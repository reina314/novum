use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    syntax::{
        BinOp,
        Expr,
        ExprKind,
        Program,
        Pattern,
        ListItem,
        IndexExpr,
        CallArg,
    },
    runtime::{
        Value,
        StructType,
        UpvalueSpec,
        FunctionProto,
        FunctionRef,
    },
    stdlib::{
        encode_class_counts,
        is_self_pattern,
    }
};

use super::{
    Chunk,
    OpCode,
};

use std::{
    rc::Rc,
    cell::RefCell,
    collections::HashMap,
};


enum PipelineStage {
    Map(Expr),
    Filter(Expr),
    Skip(usize),
    Take(usize),
}

#[derive(Clone, Copy)]
enum Binding {
    Local(u16),
    Upvalue(u16),
}

enum LValue {
    Local(u16),
    Upvalue(u16),

    Index {
        object_slot: u16,
        index_slot: u16,
    },

    Field {
        object_slot: u16,
        name: String,
    },
}

struct LoopContext {
    cleanup_target: Option<usize>,
    continue_jumps: Vec<usize>,
    break_jumps: Vec<usize>,
    local_slots: Vec<u16>,
}

type ScopeRef = Rc<RefCell<Scope>>;

struct Scope {
    parent: Option<ScopeRef>,
    locals: HashMap<String, u16>,
    upvalues: HashMap<String, u16>,
    upvalue_specs: Vec<UpvalueSpec>,
    function_boundary: bool,
}

impl Scope {
    fn new(
        parent: Option<ScopeRef>,
        function_boundary: bool,
    ) -> ScopeRef {
        Rc::new(
            RefCell::new(
                Self {
                    parent,

                    locals:
                        HashMap::new(),

                    upvalues:
                        HashMap::new(),

                    upvalue_specs:
                        Vec::new(),

                    function_boundary,
                }
            )
        )
    }
}

pub struct Compiler {
    chunk: Chunk,
    scope: ScopeRef,
    next_local_slot: u16,
    loops: Vec<LoopContext>,
    loop_local_stack: Vec<Vec<u16>>,
    function_parameters: Vec<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            scope: Scope::new(None, true,),
            next_local_slot: 0,
            loops: Vec::new(),
            loop_local_stack: Vec::new(),
            function_parameters: Vec::new(),
        }
    }

    fn new_function(
        parent: ScopeRef,
        params: &[Pattern],
    ) -> Result<Self> {
        let scope =
            Scope::new(
                Some(parent),
                true,
            );

        let mut compiler =
            Self {
                chunk: Chunk::default(),
                scope: scope.clone(),
                next_local_slot: 0,
                loops: Vec::new(),
                loop_local_stack: Vec::new(),
                function_parameters: Vec::new(),
            };

        let mut parameter_names =
            Vec::with_capacity(
                params.len()
            );

        for (
            index,
            param,
        ) in params.iter().enumerate()
        {
            let Pattern::Ident(name) =
                param
            else {
                return Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "VM currently supports only identifier parameters",
                        None,
                    )
                );
            };

            scope
                .borrow_mut()
                .locals
                .insert(
                    name.clone(),
                    index as u16,
                );

            parameter_names.push(
                name.clone()
            );
        }

        compiler.next_local_slot =
            params.len() as u16;

        compiler.function_parameters =
            parameter_names;

        Ok(compiler)
    }

    fn declare_local(
        &mut self,
        name: String,
    ) -> Result<u16> {
        {
            let scope =
                self.scope.borrow();

            if scope.locals.contains_key(
                &name
            ) {
                return Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "variable '{}' is already declared in this scope",
                            name
                        ),
                        None,
                    )
                );
            }
        }

        let slot =
            self.next_local_slot;

        self.next_local_slot += 1;

        self.scope
            .borrow_mut()
            .locals
            .insert(
                name,
                slot,
            );

        if let Some(slots) =
            self.loop_local_stack.last_mut()
        {
            slots.push(slot);
        }

        Ok(slot)
    }

    fn allocate_temp_local(
        &mut self,
    ) -> u16 {
        let slot =
            self.next_local_slot;

        self.next_local_slot += 1;

        slot
    }

    fn resolve_local(
        &self,
        name: &str,
    ) -> Option<u16> {
        let mut scope =
            Some(self.scope.clone());

        while let Some(current) =
            scope
        {
            if let Some(slot) =
                current
                    .borrow()
                    .locals
                    .get(name)
                    .copied()
            {
                return Some(slot);
            }

            let is_function_boundary =
                current
                    .borrow()
                    .function_boundary;

            if is_function_boundary {
                break;
            }

            scope =
                current
                    .borrow()
                    .parent
                    .clone();
        }

        None
    }

    fn emit_load_lvalue(
        &mut self,
        lvalue: &LValue,
    ) -> Result<()> {
        match lvalue {
            LValue::Local(slot) => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *slot as u32,
                );
            }

            LValue::Upvalue(slot) => {
                self.chunk.emit_operand(
                    OpCode::LoadUpvalue,
                    *slot as u32,
                );
            }

            LValue::Index {
                object_slot,
                index_slot,
            } => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *object_slot as u32,
                );

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *index_slot as u32,
                );

                self.chunk.emit(
                    OpCode::IndexGet
                );
            }

            LValue::Field {
                object_slot,
                name,
            } => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *object_slot as u32,
                );

                let field_constant =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                name.clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    field_constant,
                );

                self.chunk.emit(
                    OpCode::FieldGet
                );
            }
        }

        Ok(())
    }

    fn emit_store_lvalue(
        &mut self,
        lvalue: &LValue,
    ) -> Result<()> {
        match lvalue {
            LValue::Local(slot) => {
                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    *slot as u32,
                );
            }

            LValue::Upvalue(slot) => {
                self.chunk.emit_operand(
                    OpCode::StoreUpvalue,
                    *slot as u32,
                );
            }

            LValue::Index {
                object_slot,
                index_slot,
            } => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *object_slot as u32,
                );

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *index_slot as u32,
                );

                self.chunk.emit(
                    OpCode::IndexSet
                );
            }

            LValue::Field {
                object_slot,
                name,
            } => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *object_slot as u32,
                );

                let field_constant =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                name.clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    field_constant,
                );

                self.chunk.emit(
                    OpCode::FieldSet
                );
            }
        }

        Ok(())
    }

    fn emit_literal_pattern(
        &mut self,
        value_slot: u16,
        expected: Value,
    ) -> usize {
        self.chunk.emit_operand(
            OpCode::LoadLocal,
            value_slot as u32,
        );

        let constant =
            self.chunk.add_constant(
                expected
            );

        self.chunk.emit_operand(
            OpCode::Constant,
            constant,
        );

        self.chunk.emit(
            OpCode::Eq
        );

        self.chunk.emit_operand(
            OpCode::JumpIfFalse,
            0,
        )
    }

    fn emit_struct_match(
        &mut self,
        value_slot: u16,
        path: &[String],
        field_count: usize,
    ) -> Result<usize> {
        if path.len() != 1 {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "struct pattern requires a struct name",
                    None,
                )
            );
        }

        let name =
            self.chunk.add_constant(
                Value::Str(
                    Rc::new(
                        path[0].clone()
                    )
                )
            );

        self.chunk.emit_operand(
            OpCode::LoadLocal,
            value_slot as u32,
        );

        self.chunk.emit_operand(
            OpCode::Constant,
            name,
        );

        self.chunk.emit_operand(
            OpCode::MatchStruct,
            field_count as u32,
        );

        Ok(
            self.chunk.emit_operand(
                OpCode::JumpIfFalse,
                0,
            )
        )
    }

    fn enter_scope(
        &mut self,
    ) {
        let parent =
            self.scope.clone();

        self.scope =
            Scope::new(
                Some(parent),
                false,
            );
    }

    fn exit_scope(
        &mut self,
    ) {
        let parent =
            self.scope
                .borrow()
                .parent
                .clone()
                .expect(
                    "cannot exit root scope"
                );

        self.scope =
            parent;
    }

    fn find_binding(
        scope: ScopeRef,
        name: &str,
    ) -> Option<Binding> {
        let mut current =
            Some(scope);

        while let Some(scope) =
            current
        {
            let borrowed =
                scope.borrow();

            if let Some(slot) =
                borrowed.locals
                    .get(name)
                    .copied()
            {
                return Some(
                    Binding::Local(slot)
                );
            }

            if let Some(slot) =
                borrowed.upvalues
                    .get(name)
                    .copied()
            {
                return Some(
                    Binding::Upvalue(slot)
                );
            }

            let parent =
                borrowed.parent.clone();

            let boundary =
                borrowed.function_boundary;

            drop(borrowed);

            if boundary {
                return None;
            }

            current =
                parent;
        }

        None
    }

    fn find_function_scope(
        scope: ScopeRef,
    ) -> Option<ScopeRef> {
        let mut current =
            Some(scope);

        while let Some(scope) =
            current
        {
            if scope
                .borrow()
                .function_boundary
            {
                return Some(scope);
            }

            current =
                scope
                    .borrow()
                    .parent
                    .clone();
        }

        None
    }

    fn ensure_capture(
        function_scope: ScopeRef,
        name: &str,
    ) -> Option<u16> {
        if let Some(index) =
            function_scope
                .borrow()
                .upvalues
                .get(name)
                .copied()
        {
            return Some(index);
        }

        let parent =
            function_scope
                .borrow()
                .parent
                .clone()?;

        // A binding in the lexical environment of the
        // parent function can be captured directly.
        if let Some(binding) =
            Self::find_binding(
                parent.clone(),
                name,
            )
        {
            let spec =
                match binding {
                    Binding::Local(slot) =>
                        UpvalueSpec::Local(slot),

                    Binding::Upvalue(slot) =>
                        UpvalueSpec::Parent(slot),
                };

            return Some(
                Self::register_upvalue(
                    &function_scope,
                    name,
                    spec,
                )
            );
        }

        // The immediate parent function does not know the
        // variable yet. Find its function scope and make
        // that function capture it first.
        let parent_function =
            Self::find_function_scope(
                parent
            )?;

        let parent_index =
            Self::ensure_capture(
                parent_function,
                name,
            )?;

        Some(
            Self::register_upvalue(
                &function_scope,
                name,
                UpvalueSpec::Parent(
                    parent_index
                ),
            )
        )
    }

    fn register_upvalue(
        scope: &ScopeRef,
        name: &str,
        spec: UpvalueSpec,
    ) -> u16 {
        let mut scope =
            scope.borrow_mut();

        if let Some(index) =
            scope.upvalues.get(name)
        {
            return *index;
        }

        let index =
            scope.upvalue_specs.len()
                as u16;

        scope.upvalue_specs.push(
            spec
        );

        scope.upvalues.insert(
            name.to_string(),
            index,
        );

        index
    }

    fn resolve_upvalue(
        &mut self,
        name: &str,
    ) -> Option<u16> {
        let function_scope =
            Self::find_function_scope(
                self.scope.clone()
            )?;

        Self::ensure_capture(
            function_scope,
            name,
        )
    }

    pub fn finish(self) -> Chunk {
        let mut chunk =
            self.chunk;

        chunk.local_count =
            self.next_local_slot as usize;

        chunk
    }

    fn finish_function(
        self,
        arity: u16,
    ) -> FunctionProto {
        let mut chunk =
            self.chunk;

        chunk.local_count =
            self.next_local_slot as usize;

        let upvalue_specs =
            self.scope
                .borrow()
                .upvalue_specs
                .clone();

        FunctionProto {
            arity,
            parameters:
                self.function_parameters,
            chunk: Rc::new(chunk),
            upvalue_specs,
        }
    }

    fn add_call_site(
        &mut self,
        args: &[CallArg],
        method: Option<u32>,
    ) -> Result<u32> {
        if args.len() >
            u16::MAX as usize
        {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    "too many call arguments",
                    None,
                )
            );
        }

        let names =
            args.iter()
                .map(|arg|
                    arg.name.clone()
                )
                .collect();

        Ok(
            self.chunk.add_call_site(
                names,
                method,
            )
        )
    }

    fn extract_pipeline(
        &self,
        expr: &Expr,
    ) -> Result<
        Option<(
            Expr,
            Vec<PipelineStage>,
        )>
    > {
        let mut stages =
            Vec::new();

        let mut current =
            expr;

        loop {
            let ExprKind::Call(
                callee,
                args,
            ) =
                &current.kind
            else {
                break;
            };

            let ExprKind::Field {
                object,
                name,
            } =
                &callee.kind
            else {
                break;
            };

            match name.as_str() {
                "map" => {
                    if args.len() != 1
                        || args[0].name.is_some()
                    {
                        return Ok(None);
                    }

                    stages.push(
                        PipelineStage::Map(
                            args[0]
                                .value
                                .as_ref()
                                .clone()
                        )
                    );

                    current =
                        object;
                }

                "filter" => {
                    if args.len() != 1
                        || args[0].name.is_some()
                    {
                        return Ok(None);
                    }

                    stages.push(
                        PipelineStage::Filter(
                            args[0]
                                .value
                                .as_ref()
                                .clone()
                        )
                    );

                    current =
                        object;
                }

                "skip" => {
                    if args.len() != 1
                        || args[0].name.is_some()
                    {
                        return Ok(None);
                    }

                    let ExprKind::Int(
                        count
                    ) =
                        &args[0].value.kind
                    else {
                        return Ok(None);
                    };

                    if *count < 0 {
                        return Ok(None);
                    }

                    stages.push(
                        PipelineStage::Skip(
                            *count as usize
                        )
                    );

                    current =
                        object;
                }

                "take" => {
                    if args.len() != 1
                        || args[0].name.is_some()
                    {
                        return Ok(None);
                    }

                    let ExprKind::Int(
                        count
                    ) =
                        &args[0].value.kind
                    else {
                        return Ok(None);
                    };

                    if *count < 0 {
                        return Ok(None);
                    }

                    stages.push(
                        PipelineStage::Take(
                            *count as usize
                        )
                    );

                    current =
                        object;
                }

                _ => {
                    break;
                }
            }
        }

        if stages.is_empty() {
            return Ok(None);
        }

        stages.reverse();

        Ok(
            Some(
                (
                    current.clone(),
                    stages,
                )
            )
        )
    }

    fn is_fusable_pipeline_expr(
        expr: &Expr,
    ) -> bool {
        match &expr.kind {
            ExprKind::Lambda(..) => false,

            ExprKind::Return(_) =>
                false,

            ExprKind::Break |
            ExprKind::Continue =>
                false,

            ExprKind::StructDecl { .. } |
            ExprKind::ClassDecl { .. } |
            ExprKind::EnumDecl(_) |
            ExprKind::Import { .. } |
            ExprKind::Drop(_) =>
                false,

            ExprKind::Tuple(values) =>
                values.iter()
                    .all(
                        Self::is_fusable_pipeline_expr
                    ),

            ExprKind::Dict(values) =>
                values.iter()
                    .all(
                        |(_, value)|
                            Self::is_fusable_pipeline_expr(
                                value
                            )
                    ),

            ExprKind::List(items) =>
                items.iter()
                    .all(
                        |item|
                            match item {
                                ListItem::Expr(expr) =>
                                    Self::is_fusable_pipeline_expr(
                                        expr
                                    ),

                                ListItem::Range(index) =>
                                    Self::is_fusable_index_expr(
                                        index
                                    ),
                            }
                    ),

            ExprKind::TupleIndex {
                object,
                ..
            } =>
                Self::is_fusable_pipeline_expr(
                    object
                ),

            ExprKind::Binary(
                _,
                left,
                right,
            ) =>
                Self::is_fusable_pipeline_expr(
                    left
                )
                &&
                Self::is_fusable_pipeline_expr(
                    right
                ),

            ExprKind::Neg(expr) |
            ExprKind::Not(expr) |
            ExprKind::Try(expr) =>
                Self::is_fusable_pipeline_expr(
                    expr
                ),

            ExprKind::If(
                cond,
                then_branch,
                else_branch,
            ) => {
                Self::is_fusable_pipeline_expr(
                    cond
                )
                &&
                Self::is_fusable_pipeline_expr(
                    then_branch
                )
                &&
                else_branch
                    .as_ref()
                    .map(
                        |expr|
                            Self::is_fusable_pipeline_expr(
                                expr
                            )
                    )
                    .unwrap_or(true)
            }

            ExprKind::While(
                cond,
                body,
            ) =>
                Self::is_fusable_pipeline_expr(
                    cond
                )
                &&
                Self::is_fusable_pipeline_expr(
                    body
                ),

            ExprKind::For {
                iterable,
                body,
                ..
            } =>
                Self::is_fusable_pipeline_expr(
                    iterable
                )
                &&
                Self::is_fusable_pipeline_expr(
                    body
                ),

            ExprKind::Match {
                value,
                arms,
            } =>
                Self::is_fusable_pipeline_expr(
                    value
                )
                &&
                arms.iter()
                    .all(
                        |arm|
                            Self::is_fusable_pipeline_expr(
                                &arm.body
                            )
                    ),

            ExprKind::Block(exprs) =>
                exprs.iter()
                    .all(
                        Self::is_fusable_pipeline_expr
                    ),

            ExprKind::Call(
                callee,
                args,
            ) =>
                Self::is_fusable_pipeline_expr(
                    callee
                )
                &&
                args.iter()
                    .all(
                        |arg|
                            Self::is_fusable_pipeline_expr(
                                &arg.value
                            )
                    ),

            ExprKind::Index(
                object,
                index,
            ) =>
                Self::is_fusable_pipeline_expr(
                    object
                )
                &&
                Self::is_fusable_index_expr(
                    index
                ),

            ExprKind::Field {
                object,
                ..
            } =>
                Self::is_fusable_pipeline_expr(
                    object
                ),

            ExprKind::Range {
                start,
                end,
                ..
            } =>
                start
                    .as_ref()
                    .map(
                        |expr|
                            Self::is_fusable_pipeline_expr(
                                expr
                            )
                    )
                    .unwrap_or(true)
                &&
                end
                    .as_ref()
                    .map(
                        |expr|
                            Self::is_fusable_pipeline_expr(
                                expr
                            )
                    )
                    .unwrap_or(true),

            ExprKind::Let {
                value,
                ..
            } =>
                Self::is_fusable_pipeline_expr(
                    value
                ),

            ExprKind::Assign {
                target,
                value,
            } |
            ExprKind::AssignOp {
                target,
                value,
                ..
            } =>
                Self::is_fusable_pipeline_expr(
                    target
                )
                &&
                Self::is_fusable_pipeline_expr(
                    value
                ),

            ExprKind::Int(_) |
            ExprKind::Float(_) |
            ExprKind::Str(_) |
            ExprKind::Bool(_) |
            ExprKind::Ident(_) |
            ExprKind::Null |
            ExprKind::Unit =>
                true,
        }
    }

    fn is_fusable_index_expr(
        expr: &IndexExpr,
    ) -> bool {
        match expr {
            IndexExpr::Single(expr) =>
                Self::is_fusable_pipeline_expr(
                    expr
                ),

            IndexExpr::Range {
                start,
                end,
                ..
            } =>
                start
                    .as_ref()
                    .map(
                        |expr|
                            Self::is_fusable_pipeline_expr(
                                expr
                            )
                    )
                    .unwrap_or(true)
                &&
                end
                    .as_ref()
                    .map(
                        |expr|
                            Self::is_fusable_pipeline_expr(
                                expr
                            )
                    )
                    .unwrap_or(true),

            IndexExpr::Tuple(indices) =>
                indices.iter()
                    .all(
                        Self::is_fusable_index_expr
                    ),
        }
    }

    pub fn compile_program(
        &mut self,
        program: &Program,
    ) -> Result<Chunk> {
        for (
            index,
            expr,
        ) in program.statements.iter().enumerate()
        {
            self.compile_expr(
                expr
            )?;

            if index + 1
                != program.statements.len()
            {
                self.chunk.emit(
                    OpCode::Pop
                );
            }
        }

        self.chunk.emit(
            OpCode::Halt
        );

        self.chunk.local_count =
            self.next_local_slot as usize;

        Ok(
            std::mem::take(
                &mut self.chunk
            )
        )
    }

    pub fn compile(
        mut self,
        program: &Program,
    ) -> Result<Chunk> {
        for (
            index,
            expr,
        ) in program.statements.iter().enumerate()
        {
            self.compile_expr(
                expr
            )?;

            if index + 1
                != program.statements.len()
            {
                self.chunk.emit(
                    OpCode::Pop
                );
            }
        }

        self.chunk.emit(
            OpCode::Halt
        );

        self.chunk.local_count =
            self.next_local_slot as usize;

        Ok(self.chunk)
    }

    fn compile_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<()> {
        match &expr.kind {
            ExprKind::Int(value) => {
                let index =
                    self.chunk.add_constant(
                        Value::Int(*value)
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    index,
                );
            }

            ExprKind::Float(value) => {
                let index =
                    self.chunk.add_constant(
                        Value::Float(*value)
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    index,
                );
            }

            ExprKind::Bool(value) => {
                let index =
                    self.chunk.add_constant(
                        Value::Bool(*value)
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    index,
                );
            }

            ExprKind::Str(value) => {
                let index =
                    self.chunk.add_constant(
                        Value::Str(
                            std::rc::Rc::new(
                                value.clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    index,
                );
            }

            ExprKind::Neg(inner) => {
                self.compile_expr(
                    inner
                )?;

                self.chunk.emit(
                    OpCode::Neg
                );
            }

            ExprKind::Not(inner) => {
                self.compile_expr(
                    inner
                )?;

                self.chunk.emit(
                    OpCode::Not
                );
            }

            ExprKind::Binary(
                op,
                left,
                right,
            ) => {
                self.compile_expr(
                    left
                )?;

                self.compile_expr(
                    right
                )?;

                self.compile_binop(
                    *op
                )?;
            }

            ExprKind::Tuple(elements) => {
                if elements.len() > u16::MAX as usize {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "tuple is too large",
                            None,
                        )
                    );
                }

                for element in elements {
                    self.compile_expr(
                        element
                    )?;
                }

                self.chunk.emit_operand(
                    OpCode::NewTuple,
                    elements.len() as u32,
                );
            }

            ExprKind::TupleIndex {
                object,
                index,
            } => {
                self.compile_expr(
                    object
                )?;

                let constant =
                    self.chunk.add_constant(
                        Value::Int(
                            *index as i64
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    constant,
                );

                self.chunk.emit(
                    OpCode::IndexGet
                );
            }

            ExprKind::EnumDecl(def) => {
                let mut enum_def =
                    crate::runtime::EnumDef::new(
                        def.name.clone()
                    );

                for variant in
                    &def.variants
                {
                    enum_def
                        .add_variant(
                            variant.name.clone(),
                            variant.fields.len(),
                        )
                        .map_err(|message| {
                            Error::new(
                                ErrorKind::Name,
                                message,
                                None,
                            )
                        })?;
                }

                let enum_slot =
                    self.declare_local(
                        def.name.clone()
                    )?;

                let constant =
                    self.chunk.add_constant(
                        Value::Enum(
                            Rc::new(enum_def)
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    constant,
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    enum_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                // Enum declarations do not produce a value.
                self.chunk.emit(
                    OpCode::Unit
                );
            }

            ExprKind::StructDecl {
                name,
                fields,
                methods,
                ..
            } => {
                if !methods.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "struct methods are not supported; use class",
                            None,
                        )
                    );
                }

                let field_names =
                    fields
                        .iter()
                        .map(
                            |(name, _)| name.clone()
                        )
                        .collect::<Vec<_>>();

                if fields.iter()
                    .any(|(_, default)| default.is_some())
                {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "struct fields cannot have default values; use class",
                            None,
                        )
                    );
                }

                let ty =
                    StructType::new(
                        name.clone(),
                        field_names,
                    );

                let slot =
                    self.declare_local(
                        name.clone()
                    )?;

                let constant =
                    self.chunk.add_constant(
                        Value::StructType(
                            Rc::new(ty)
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    constant,
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                self.chunk.emit(
                    OpCode::Unit
                );
            }

            ExprKind::ClassDecl {
                name,
                fields,
                methods,
                ..
            } => {
                let class_name_constant =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                name.clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    class_name_constant,
                );

                // Compile field names and per-instance default expressions.
                for (
                    field_name,
                    default,
                ) in fields
                {
                    let field_name_constant =
                        self.chunk.add_constant(
                            Value::Str(
                                Rc::new(
                                    field_name.clone()
                                )
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        field_name_constant,
                    );

                    match default {
                        Some(expr) => {
                            let proto =
                                self.compile_zero_arg_proto(
                                    expr
                                )?;

                            let proto_constant =
                                self.chunk.add_constant(
                                    Value::FunctionProto(
                                        proto
                                    )
                                );

                            self.chunk.emit_operand(
                                OpCode::Closure,
                                proto_constant,
                            );
                        }

                        None => {
                            self.chunk.emit(
                                OpCode::Unit
                            );
                        }
                    }
                }

                // Compile methods into closures.
                for (
                    method_name,
                    method,
                ) in methods
                {
                    let ExprKind::Lambda(
                        params,
                        body,
                    ) =
                        &method.kind
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                format!(
                                    "class method '{}' must be a lambda",
                                    method_name
                                ),
                                None,
                            )
                        );
                    };

                    if params.first()
                        .map(is_self_pattern)
                        != Some(true)
                    {
                        return Err(
                            Error::new(
                                ErrorKind::Name,
                                format!(
                                    "method '{}' must have 'self' as its first parameter",
                                    method_name
                                ),
                                None,
                            )
                        );
                    }

                    let method_name_constant =
                        self.chunk.add_constant(
                            Value::Str(
                                Rc::new(
                                    method_name.clone()
                                )
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        method_name_constant,
                    );

                    let proto =
                        self.compile_lambda_proto(
                            params,
                            body,
                        )?;

                    let proto_constant =
                        self.chunk.add_constant(
                            Value::FunctionProto(
                                proto
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Closure,
                        proto_constant,
                    );
                }

                let operand =
                    encode_class_counts(
                        fields.len(),
                        methods.len(),
                    );

                self.chunk.emit_operand(
                    OpCode::NewClass,
                    operand,
                );

                // Bind the resulting Class value to its declared name.
                let class_slot =
                    self.declare_local(
                        name.clone()
                    )?;

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    class_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                // A declaration evaluates to Unit.
                self.chunk.emit(
                    OpCode::Unit
                );
            }

            ExprKind::List(items) => {
                let explicit_capacity =
                    items
                        .iter()
                        .filter(
                            |item| {
                                matches!(
                                    item,
                                    ListItem::Expr(_)
                                )
                            }
                        )
                        .count();

                self.chunk.emit_operand(
                    OpCode::NewList,
                    explicit_capacity as u32,
                );

                for item in items {
                    match item {
                        ListItem::Expr(expr) => {
                            self.compile_expr(
                                expr
                            )?;

                            self.chunk.emit(
                                OpCode::ListAppend
                            );
                        }

                        ListItem::Range(
                            IndexExpr::Range {
                                start,
                                end,
                                inclusive,
                            }
                        ) => {
                            let Some(start) =
                                start
                            else {
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "list range requires a start value",
                                        None,
                                    )
                                );
                            };

                            let Some(end) =
                                end
                            else {
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "list range requires an end value",
                                        None,
                                    )
                                );
                            };

                            self.compile_expr(
                                start
                            )?;

                            self.compile_expr(
                                end
                            )?;

                            self.chunk.emit_operand(
                                OpCode::ListExtendRange,
                                if *inclusive {
                                    1
                                } else {
                                    0
                                },
                            );
                        }

                        ListItem::Range(_) => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "unsupported list range expression",
                                    None,
                                )
                            );
                        }
                    }
                }
            }

            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let Some(start) =
                    start
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently requires a range start",
                            None,
                        )
                    );
                };

                let Some(end) =
                    end
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently requires a range end",
                            None,
                        )
                    );
                };

                self.compile_expr(
                    start
                )?;

                self.compile_expr(
                    end
                )?;

                self.chunk.emit_operand(
                    OpCode::NewRange,
                    if *inclusive {
                        1
                    } else {
                        0
                    },
                );
            }

            ExprKind::Index(
                object,
                IndexExpr::Single(index),
            ) => {
                self.compile_expr(
                    object
                )?;

                self.compile_expr(
                    index
                )?;

                self.chunk.emit(
                    OpCode::IndexGet
                );
            }

            ExprKind::Field {
                object,
                name,
            } => {
                self.compile_expr(
                    object
                )?;

                let constant =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                name.clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    constant,
                );

                self.chunk.emit(
                    OpCode::FieldGet
                );
            }

            ExprKind::Let {
                pattern,
                value,
                ..
            } => {
                self.compile_expr(
                    value
                )?;

                let value_slot =
                    self.allocate_temp_local();

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    value_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                let failures =
                    self.compile_pattern(
                        value_slot,
                        pattern,
                    )?;

                let success_jump =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                let failure_target =
                    self.chunk.code.len();

                for jump in failures {
                    self.chunk.patch_operand(
                        jump,
                        failure_target as u32,
                    );
                }

                self.chunk.emit(
                    OpCode::PatternFail
                );

                let end =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    success_jump,
                    end as u32,
                );

                self.chunk.emit(
                    OpCode::Unit
                );
            }

            ExprKind::Assign {
                target,
                value,
            } => {
                let lvalue =
                    self.compile_lvalue(
                        target
                    )?;

                self.compile_expr(
                    value
                )?;

                self.emit_store_lvalue(
                    &lvalue
                )?;
            }

            ExprKind::AssignOp {
                target,
                op,
                value,
            } => {
                let lvalue =
                    self.compile_lvalue(
                        target
                    )?;

                self.emit_load_lvalue(
                    &lvalue
                )?;

                self.compile_expr(
                    value
                )?;

                self.compile_binop(
                    *op
                )?;

                self.emit_store_lvalue(
                    &lvalue
                )?;
            }

            ExprKind::Ident(name) => {
                if let Some(slot) =
                    self.resolve_local(name)
                {
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        slot as u32,
                    );
                } else if let Some(slot) =
                    self.resolve_upvalue(name)
                {
                    self.chunk.emit_operand(
                        OpCode::LoadUpvalue,
                        slot as u32,
                    );
                } else {
                    return Err(
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "{} is undefined",
                                name
                            ),
                            None,
                        )
                    );
                }
            }

            ExprKind::If(
                cond,
                then_branch,
                else_branch,
            ) => {
                self.compile_expr(
                    cond
                )?;

                let jump_if_false =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                self.compile_expr(
                    then_branch
                )?;

                let jump_end =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                let else_start =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    jump_if_false,
                    else_start as u32,
                );

                match else_branch {
                    Some(else_branch) => {
                        self.compile_expr(
                            else_branch
                        )?;
                    }

                    None => {
                        self.chunk.emit(
                            OpCode::Unit
                        );
                    }
                }

                let end =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    jump_end,
                    end as u32,
                );
            }

            ExprKind::Match {
                value,
                arms,
            } => {
                self.compile_expr(
                    value
                )?;

                let value_slot =
                    self.allocate_temp_local();

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    value_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                let mut end_jumps =
                    Vec::with_capacity(
                        arms.len()
                    );

                for arm in arms {
                    self.enter_scope();

                    // Pattern failure jumps are intentionally left
                    // unresolved until the arm body has been compiled.
                    let failures =
                        self.compile_pattern(
                            value_slot,
                            &arm.pattern,
                        )?;

                    self.compile_expr(
                        &arm.body
                    )?;

                    let end_jump =
                        self.chunk.emit_operand(
                            OpCode::Jump,
                            0,
                        );

                    end_jumps.push(
                        end_jump
                    );

                    // If the pattern failed, continue with the next arm.
                    let next_arm =
                        self.chunk.code.len();

                    for jump in failures {
                        self.chunk.patch_operand(
                            jump,
                            next_arm as u32,
                        );
                    }

                    self.exit_scope();
                }

                // No arm matched.
                self.chunk.emit(
                    OpCode::PatternFail
                );

                let end =
                    self.chunk.code.len();

                for jump in end_jumps {
                    self.chunk.patch_operand(
                        jump,
                        end as u32,
                    );
                }
            }

            ExprKind::While(
                cond,
                body,
            ) => {
                // Store the value of the most recently
                // evaluated iteration here.
                let result_slot =
                    self.allocate_temp_local();

                // A while expression that executes zero times
                // evaluates to Unit.
                self.chunk.emit(
                    OpCode::Unit
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    result_slot as u32,
                );

                let condition_target =
                    self.chunk.code.len();

                let loop_index =
                    self.loops.len();

                self.loops.push(
                    LoopContext {
                        cleanup_target:
                            None,

                        continue_jumps:
                            Vec::new(),

                        break_jumps:
                            Vec::new(),

                        local_slots:
                            Vec::new(),
                    }
                );

                self.loop_local_stack.push(
                    Vec::new()
                );

                self.compile_expr(cond)?;

                let exit =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                self.compile_expr(body)?;

                // Save the current iteration's value.
                self.chunk.emit(
                    OpCode::Dup
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    result_slot as u32,
                );

                // Discard the copy that was only used
                // for the expression result storage.
                self.chunk.emit(
                    OpCode::Pop
                );

                let cleanup_target =
                    self.chunk.code.len();

                self.loops[loop_index]
                    .cleanup_target =
                    Some(cleanup_target);

                let local_slots =
                    self.loop_local_stack
                        .pop()
                        .unwrap();

                self.loops[loop_index]
                    .local_slots =
                    local_slots;

                for jump in
                    self.loops[loop_index]
                        .continue_jumps
                        .iter()
                {
                    self.chunk.patch_operand(
                        *jump,
                        cleanup_target as u32,
                    );
                }

                for slot in
                    &self.loops[loop_index]
                        .local_slots
                {
                    self.chunk.emit_operand(
                        OpCode::ResetLocal,
                        *slot as u32,
                    );
                }

                self.chunk.emit_operand(
                    OpCode::Jump,
                    condition_target as u32,
                );

                let loop_end =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    exit,
                    loop_end as u32,
                );

                let context =
                    self.loops.pop().unwrap();

                for jump in
                    context.break_jumps
                {
                    self.chunk.patch_operand(
                        jump,
                        loop_end as u32,
                    );
                }

                // Return the last iteration's value.
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    result_slot as u32,
                );
            }

            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                let iterator_slot =
                    self.allocate_temp_local();

                let result_slot =
                    self.allocate_temp_local();

                // Initialize the result of a zero-iteration loop.
                self.chunk.emit(
                    OpCode::Unit
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    result_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                // iterable -> Iterator
                self.compile_expr(
                    iterable
                )?;

                self.chunk.emit(
                    OpCode::IteratorFrom
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    iterator_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                let loop_start =
                    self.chunk.code.len();

                let loop_index =
                    self.loops.len();

                self.loops.push(
                    LoopContext {
                        cleanup_target:
                            None,

                        continue_jumps:
                            Vec::new(),

                        break_jumps:
                            Vec::new(),

                        local_slots:
                            Vec::new(),
                    }
                );

                self.loop_local_stack.push(
                    Vec::new()
                );

                // Create the loop binding inside the loop scope.
                self.enter_scope();

                let loop_slot =
                    match pattern {
                        Pattern::Ident(name) => {
                            self.declare_local(
                                name.clone()
                            )?
                        }

                        Pattern::Wildcard => {
                            // Still consume the item, but do not
                            // expose a binding.
                            self.allocate_temp_local()
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "VM currently supports only identifier and wildcard for-loop patterns",
                                    None,
                                )
                            );
                        }
                    };

                // Fetch the next item.
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    iterator_slot as u32,
                );

                self.chunk.emit(
                    OpCode::IteratorNext
                );

                let exit =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                // The item remains on the stack after JumpIfFalse.
                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    loop_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                // Compile the loop body.
                self.compile_expr(
                    body
                )?;

                // Save the value of the completed iteration.
                self.chunk.emit(
                    OpCode::Dup
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    result_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                self.exit_scope();

                let cleanup_target =
                    self.chunk.code.len();

                self.loops[loop_index]
                    .cleanup_target =
                    Some(cleanup_target);

                let local_slots =
                    self.loop_local_stack
                        .pop()
                        .unwrap();

                self.loops[loop_index]
                    .local_slots =
                    local_slots;

                for jump in
                    self.loops[loop_index]
                        .continue_jumps
                        .iter()
                {
                    self.chunk.patch_operand(
                        *jump,
                        cleanup_target as u32,
                    );
                }

                // End the current iteration.
                for slot in
                    &self.loops[loop_index]
                        .local_slots
                {
                    self.chunk.emit_operand(
                        OpCode::ResetLocal,
                        *slot as u32,
                    );
                }

                self.chunk.emit_operand(
                    OpCode::Jump,
                    loop_start as u32,
                );

                let loop_end =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    exit,
                    loop_end as u32,
                );

                let context =
                    self.loops.pop().unwrap();

                for jump in
                    context.break_jumps
                {
                    self.chunk.patch_operand(
                        jump,
                        loop_end as u32,
                    );
                }

                // IteratorNext leaves Unit on the stack when
                // the loop terminates normally.
                self.chunk.emit(
                    OpCode::Pop
                );

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    result_slot as u32,
                );
            }

            ExprKind::Break => {
                let Some(loop_context) =
                    self.loops.last_mut()
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Control,
                            "break outside loop",
                            None,
                        )
                    );
                };

                let jump =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                loop_context
                    .break_jumps
                    .push(jump);
            }

            ExprKind::Continue => {
                let Some(loop_context) =
                    self.loops.last_mut()
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Control,
                            "continue outside loop",
                            None,
                        )
                    );
                };

                let jump =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                loop_context
                    .continue_jumps
                    .push(jump);
            }

            ExprKind::Lambda(
                params,
                body,
            ) => {
                let function =
                    self.compile_lambda_proto(
                        params,
                        body,
                    )?;

                let index =
                    self.chunk.add_constant(
                        Value::FunctionProto(
                            function
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Closure,
                    index,
                );
            }

            ExprKind::Call(
                callee,
                args,
            ) => {
                if self.try_compile_fused_collect(
                    callee,
                    args,
                )? {
                    return Ok(());
                }

                // receiver.method(...)
                if let ExprKind::Field {
                    object,
                    name,
                } = &callee.kind
                {
                    self.compile_expr(
                        object
                    )?;

                    let method_index =
                        self.chunk.add_constant(
                            Value::Str(
                                Rc::new(
                                    name.clone()
                                )
                            )
                        );

                    for arg in args {
                        self.compile_expr(
                            &arg.value
                        )?;
                    }

                    let call_site =
                        self.add_call_site(
                            args,
                            Some(method_index),
                        )?;

                    self.chunk.emit_operand(
                        OpCode::InvokeMethod,
                        call_site,
                    );

                    return Ok(());
                }

                // Intrinsics
                if let ExprKind::Ident(name) =
                    &callee.kind
                {
                    match name.as_str() {
                        "iter" => {
                            if args.len() != 1 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Arity,
                                        "iter() expects exactly one argument",
                                        None,
                                    )
                                );
                            }

                            self.compile_expr(
                                &args[0].value
                            )?;

                            self.chunk.emit(
                                OpCode::IteratorFrom
                            );

                            return Ok(());
                        }

                        "next" => {
                            if args.len() != 1 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Arity,
                                        "next() expects exactly one argument",
                                        None,
                                    )
                                );
                            }

                            self.compile_expr(
                                &args[0].value
                            )?;

                            self.chunk.emit(
                                OpCode::IteratorNext
                            );

                            return Ok(());
                        }

                        _ => {}
                    }
                }

                // Normal function call.
                self.compile_expr(
                    callee
                )?;

                for arg in args {
                    self.compile_expr(
                        &arg.value
                    )?;
                }

                let call_site =
                    self.add_call_site(
                        args,
                        None,
                    )?;

                self.chunk.emit_operand(
                    OpCode::Call,
                    call_site,
                );
            }

            ExprKind::Block(exprs) => {
                self.enter_scope();

                if exprs.is_empty() {
                    self.chunk.emit(
                        OpCode::Unit
                    );
                } else {
                    for (
                        index,
                        expr,
                    ) in exprs.iter().enumerate()
                    {
                        self.compile_expr(
                            expr
                        )?;

                        if index + 1
                            != exprs.len()
                        {
                            self.chunk.emit(
                                OpCode::Pop
                            );
                        }
                    }
                }

                self.exit_scope();
            }

            _ => {
                return Err(
                    Error::new(
                        ErrorKind::Runtime,
                        format!(
                            "VM compiler does not support {:?}",
                            expr.kind
                        ),
                        None,
                    )
                );
            }
        }

        Ok(())
    }

    fn compile_binop(
        &mut self,
        op: BinOp,
    ) -> Result<()> {
        let opcode =
            match op {
                BinOp::Add =>
                    OpCode::Add,

                BinOp::Sub =>
                    OpCode::Sub,

                BinOp::Mul =>
                    OpCode::Mul,

                BinOp::Div =>
                    OpCode::Div,

                BinOp::Mod =>
                    OpCode::Mod,

                BinOp::Pow =>
                    OpCode::Pow,

                BinOp::MatMul =>
                    OpCode::MatMul,

                BinOp::Eq =>
                    OpCode::Eq,

                BinOp::Neq =>
                    OpCode::Neq,

                BinOp::Lt =>
                    OpCode::Lt,

                BinOp::Leq =>
                    OpCode::Leq,

                BinOp::Gt =>
                    OpCode::Gt,

                BinOp::Geq =>
                    OpCode::Geq,

                _ =>
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            format!(
                                "unsupported binary operator: {}",
                                op
                            ),
                            None,
                        )
                    ),
            };

        self.chunk.emit(opcode);

        Ok(())
    }

    fn compile_lvalue(
        &mut self,
        target: &Expr,
    ) -> Result<LValue> {
        match &target.kind {
            ExprKind::Ident(name) => {
                if let Some(slot) =
                    self.resolve_local(name)
                {
                    return Ok(
                        LValue::Local(slot)
                    );
                }

                if let Some(slot) =
                    self.resolve_upvalue(name)
                {
                    return Ok(
                        LValue::Upvalue(slot)
                    );
                }

                let slot =
                    self.declare_local(
                        name.clone()
                    )?;

                Ok(
                    LValue::Local(slot)
                )
            }

            ExprKind::Index(
                object,
                index,
            ) => {
                let IndexExpr::Single(index) =
                    index
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently supports only single-index assignment",
                            None,
                        )
                    );
                };

                let object_slot =
                    self.allocate_temp_local();

                self.compile_expr(
                    object
                )?;

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    object_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                let index_slot =
                    self.allocate_temp_local();

                self.compile_expr(
                    index
                )?;

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    index_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                Ok(
                    LValue::Index {
                        object_slot,
                        index_slot,
                    }
                )
            }

            ExprKind::Field {
                object,
                name,
            } => {
                self.compile_expr(
                    object
                )?;

                let object_slot =
                    self.allocate_temp_local();

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    object_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                Ok(
                    LValue::Field {
                        object_slot,
                        name: name.clone(),
                    }
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "invalid assignment target",
                        None,
                    )
                )
            }
        }
    }

    fn compile_lambda_proto(
        &self,
        params: &[Pattern],
        body: &Expr,
    ) -> Result<FunctionRef> {
        let mut compiler =
            Compiler::new_function(
                self.scope.clone(),
                params,
            )?;

        compiler.compile_expr(
            body
        )?;

        compiler.chunk.emit(
            OpCode::Return
        );

        Ok(
            Rc::new(
                compiler.finish_function(
                    params.len() as u16
                )
            )
        )
    }

    fn compile_zero_arg_proto(
        &self,
        body: &Expr,
    ) -> Result<FunctionRef> {
        let mut compiler =
            Compiler::new_function(
                self.scope.clone(),
                &[],
            )?;

        compiler.compile_expr(
            body
        )?;

        compiler.chunk.emit(
            OpCode::Return
        );

        Ok(
            Rc::new(
                compiler.finish_function(
                    0
                )
            )
        )
    }

    fn compile_pattern(
        &mut self,
        value_slot: u16,
        pattern: &Pattern,
    ) -> Result<Vec<usize>> {
        let mut failures =
            Vec::new();

        match pattern {
            Pattern::Wildcard => {}

            Pattern::Ident(name) => {
                let slot =
                    self.declare_local(
                        name.clone()
                    )?;

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    value_slot as u32,
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );
            }

            Pattern::Int(value) => {
                failures.push(
                    self.emit_literal_pattern(
                        value_slot,
                        Value::Int(*value),
                    )
                );
            }

            Pattern::Float(value) => {
                failures.push(
                    self.emit_literal_pattern(
                        value_slot,
                        Value::Float(*value),
                    )
                );
            }

            Pattern::Bool(value) => {
                failures.push(
                    self.emit_literal_pattern(
                        value_slot,
                        Value::Bool(*value),
                    )
                );
            }

            Pattern::Str(value) => {
                failures.push(
                    self.emit_literal_pattern(
                        value_slot,
                        Value::Str(
                            Rc::new(
                                value.clone()
                            )
                        ),
                    )
                );
            }

            Pattern::Tuple(patterns) => {
                let mut element_slots =
                    Vec::with_capacity(
                        patterns.len()
                    );

                for index in
                    0..patterns.len()
                {
                    let element_slot =
                        self.allocate_temp_local();

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        value_slot as u32,
                    );

                    let index_constant =
                        self.chunk.add_constant(
                            Value::Int(
                                index as i64
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        index_constant,
                    );

                    self.chunk.emit(
                        OpCode::IndexGet
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        element_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    element_slots.push(
                        element_slot
                    );
                }

                for (
                    pattern,
                    slot,
                ) in patterns
                    .iter()
                    .zip(
                        element_slots
                    )
                {
                    failures.extend(
                        self.compile_pattern(
                            slot,
                            pattern,
                        )?
                    );
                }
            }

            Pattern::List(patterns) => {
                let mut element_slots =
                    Vec::with_capacity(
                        patterns.len()
                    );

                for index in
                    0..patterns.len()
                {
                    let element_slot =
                        self.allocate_temp_local();

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        value_slot as u32,
                    );

                    let index_constant =
                        self.chunk.add_constant(
                            Value::Int(
                                index as i64
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        index_constant,
                    );

                    self.chunk.emit(
                        OpCode::IndexGet
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        element_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    element_slots.push(
                        element_slot
                    );
                }

                for (
                    pattern,
                    slot,
                ) in patterns
                    .iter()
                    .zip(
                        element_slots
                    )
                {
                    failures.extend(
                        self.compile_pattern(
                            slot,
                            pattern,
                        )?
                    );
                }
            }

            Pattern::Enum {
                path,
                fields,
            } => {
                if path.len() != 2 {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "enum pattern requires Enum.Variant",
                            None,
                        )
                    );
                }

                let enum_name =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                path[0].clone()
                            )
                        )
                    );

                let variant =
                    self.chunk.add_constant(
                        Value::Str(
                            Rc::new(
                                path[1].clone()
                            )
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    value_slot as u32,
                );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    enum_name,
                );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    variant,
                );

                self.chunk.emit_operand(
                    OpCode::MatchEnum,
                    fields.len() as u32,
                );

                let failure =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                for (
                    index,
                    pattern,
                ) in fields.iter().enumerate()
                {
                    let field_slot =
                        self.allocate_temp_local();

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        value_slot as u32,
                    );

                    let index_constant =
                        self.chunk.add_constant(
                            Value::Int(
                                index as i64
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        index_constant,
                    );

                    self.chunk.emit(
                        OpCode::EnumFieldGet
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        field_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    failures.extend(
                        self.compile_pattern(
                            field_slot,
                            pattern,
                        )?
                    );
                }

                failures.push(
                    failure
                );
            }
        
            Pattern::Struct {
                path,
                fields,
            } => {
                let failure =
                    self.emit_struct_match(
                        value_slot,
                        path,
                        fields.len(),
                    )?;

                failures.push(
                    failure
                );

                for (
                    name,
                    pattern,
                ) in fields
                {
                    let field_slot =
                        self.allocate_temp_local();

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        value_slot as u32,
                    );

                    let name_constant =
                        self.chunk.add_constant(
                            Value::Str(
                                Rc::new(
                                    name.clone()
                                )
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        name_constant,
                    );

                    self.chunk.emit(
                        OpCode::StructFieldGet
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        field_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    failures.extend(
                        self.compile_pattern(
                            field_slot,
                            pattern,
                        )?
                    );
                }
            }
        }

        Ok(failures)
    }

    fn compile_fused_pipeline(
        &mut self,
        source: &Expr,
        stages: &[PipelineStage],
    ) -> Result<()> {
        /*
        * --------------------------------------------------------
        * 1. Allocate pipeline state
        * --------------------------------------------------------
        */

        let item_slot =
            self.allocate_temp_local();

        let result_slot =
            self.allocate_temp_local();

        let mut stage_state_slots =
            vec![None; stages.len()];

        for (
            index,
            stage,
        ) in stages.iter().enumerate()
        {
            match stage {
                PipelineStage::Skip(count)
                |
                PipelineStage::Take(count) => {
                    let slot =
                        self.allocate_temp_local();

                    let constant =
                        self.chunk.add_constant(
                            Value::Int(
                                *count as i64
                            )
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        constant,
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    stage_state_slots[index] =
                        Some(slot);
                }

                PipelineStage::Map(_)
                |
                PipelineStage::Filter(_) => {}
            }
        }

        /*
        * --------------------------------------------------------
        * 2. Allocate result list
        * --------------------------------------------------------
        */

        self.chunk.emit_operand(
            OpCode::NewList,
            0,
        );

        self.chunk.emit_operand(
            OpCode::StoreLocal,
            result_slot as u32,
        );

        self.chunk.emit(
            OpCode::Pop
        );

        /*
        * --------------------------------------------------------
        * 3. Lower source
        *
        * Range is directly lowered to a loop.
        * Everything else uses IteratorFrom.
        * --------------------------------------------------------
        */

        let range =
            match &source.kind {
                ExprKind::Range {
                    start:
                        Some(start),
                    end:
                        Some(end),
                    inclusive,
                } => {
                    Some((
                        start.as_ref(),
                        end.as_ref(),
                        *inclusive,
                    ))
                }

                _ => None,
            };

        let (
            range_current_slot,
            range_end_slot,
            iterator_slot,
            range_inclusive,
        ) =
            match range {
                Some((
                    start,
                    end,
                    inclusive,
                )) => {
                    let current_slot =
                        self.allocate_temp_local();

                    self.compile_expr(
                        start
                    )?;

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        current_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    let end_slot =
                        self.allocate_temp_local();

                    self.compile_expr(
                        end
                    )?;

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        end_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    (
                        Some(current_slot),
                        Some(end_slot),
                        None,
                        inclusive,
                    )
                }

                None => {
                    self.compile_expr(
                        source
                    )?;

                    self.chunk.emit(
                        OpCode::IteratorFrom
                    );

                    let slot =
                        self.allocate_temp_local();

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    (
                        None,
                        None,
                        Some(slot),
                        false,
                    )
                }
            };

        /*
        * --------------------------------------------------------
        * 4. Main loop
        * --------------------------------------------------------
        */

        let loop_start =
            self.chunk.code.len();

        let source_end_jump =
            match (
                range_current_slot,
                range_end_slot,
            ) {
                (
                    Some(current),
                    Some(end),
                ) => {
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        current as u32,
                    );

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        end as u32,
                    );

                    self.chunk.emit(
                        if range_inclusive {
                            OpCode::Leq
                        } else {
                            OpCode::Lt
                        }
                    );

                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    )
                }

                _ => {
                    let iterator =
                        iterator_slot.expect(
                            "iterator slot missing"
                        );

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        iterator as u32,
                    );

                    self.chunk.emit(
                        OpCode::IteratorNext
                    );

                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    )
                }
            };

        /*
        * --------------------------------------------------------
        * 5. Load current item
        * --------------------------------------------------------
        */

        match range_current_slot {
            Some(current) => {
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    current as u32,
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    item_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );

                /*
                * current += 1
                *
                * This is done before the pipeline stages so
                * every next-item jump starts from the next source
                * element.
                */
                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    current as u32,
                );

                let one =
                    self.chunk.add_constant(
                        Value::Int(1)
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
                    one,
                );

                self.chunk.emit(
                    OpCode::Add
                );

                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    current as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );
            }

            None => {
                /*
                * IteratorNext:
                *
                *     item
                *     bool
                *
                * JumpIfFalse consumes bool.
                */
                self.chunk.emit_operand(
                    OpCode::StoreLocal,
                    item_slot as u32,
                );

                self.chunk.emit(
                    OpCode::Pop
                );
            }
        }

        /*
        * --------------------------------------------------------
        * 6. Stage compilation
        * --------------------------------------------------------
        */

        let mut next_item_jumps =
            Vec::new();

        let mut end_jumps =
            Vec::new();

        for (
            index,
            stage,
        ) in stages.iter().enumerate()
        {
            match stage {
                /*
                * map(|x| expr)
                *
                * compile_pipeline_lambda() directly emits
                * expr bytecode into THIS function.
                */
                PipelineStage::Map(
                    lambda
                ) => {
                    self.compile_pipeline_lambda(
                        lambda,
                        item_slot,
                    )?;

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        item_slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );
                }

                /*
                * filter(|x| expr)
                *
                * The body leaves Bool on the stack.
                */
                PipelineStage::Filter(
                    lambda
                ) => {
                    self.compile_pipeline_lambda(
                        lambda,
                        item_slot,
                    )?;

                    let next =
                        self.chunk.emit_operand(
                            OpCode::JumpIfFalse,
                            0,
                        );

                    next_item_jumps.push(
                        next
                    );
                }

                /*
                * skip(n)
                */
                PipelineStage::Skip(
                    _
                ) => {
                    let slot =
                        stage_state_slots[index]
                            .expect(
                                "missing skip state slot"
                            );

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        slot as u32,
                    );

                    let zero =
                        self.chunk.add_constant(
                            Value::Int(0)
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        zero,
                    );

                    self.chunk.emit(
                        OpCode::Gt
                    );

                    let done =
                        self.chunk.emit_operand(
                            OpCode::JumpIfFalse,
                            0,
                        );

                    /*
                    * remaining -= 1
                    */
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        slot as u32,
                    );

                    let one =
                        self.chunk.add_constant(
                            Value::Int(1)
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        one,
                    );

                    self.chunk.emit(
                        OpCode::Sub
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );

                    let next =
                        self.chunk.emit_operand(
                            OpCode::Jump,
                            0,
                        );

                    next_item_jumps.push(
                        next
                    );

                    let stage_body =
                        self.chunk.code.len();

                    self.chunk.patch_operand(
                        done,
                        stage_body as u32,
                    );
                }

                /*
                * take(n)
                */
                PipelineStage::Take(
                    _
                ) => {
                    let slot =
                        stage_state_slots[index]
                            .expect(
                                "missing take state slot"
                            );

                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        slot as u32,
                    );

                    let zero =
                        self.chunk.add_constant(
                            Value::Int(0)
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        zero,
                    );

                    self.chunk.emit(
                        OpCode::Eq
                    );

                    let continue_stage =
                        self.chunk.emit_operand(
                            OpCode::JumpIfFalse,
                            0,
                        );

                    /*
                    * remaining == 0
                    * -> terminate the entire pipeline.
                    */
                    let end =
                        self.chunk.emit_operand(
                            OpCode::Jump,
                            0,
                        );

                    end_jumps.push(
                        end
                    );

                    let stage_body =
                        self.chunk.code.len();

                    self.chunk.patch_operand(
                        continue_stage,
                        stage_body as u32,
                    );

                    /*
                    * remaining -= 1
                    */
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        slot as u32,
                    );

                    let one =
                        self.chunk.add_constant(
                            Value::Int(1)
                        );

                    self.chunk.emit_operand(
                        OpCode::Constant,
                        one,
                    );

                    self.chunk.emit(
                        OpCode::Sub
                    );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );

                    self.chunk.emit(
                        OpCode::Pop
                    );
                }
            }
        }

        /*
        * --------------------------------------------------------
        * 7. Append accepted item
        * --------------------------------------------------------
        */

        self.chunk.emit_operand(
            OpCode::LoadLocal,
            result_slot as u32,
        );

        self.chunk.emit_operand(
            OpCode::LoadLocal,
            item_slot as u32,
        );

        self.chunk.emit(
            OpCode::ListAppend
        );

        /*
        * --------------------------------------------------------
        * 8. Next source item
        * --------------------------------------------------------
        */

        self.chunk.emit_operand(
            OpCode::Jump,
            loop_start as u32,
        );

        /*
        * --------------------------------------------------------
        * 9. Resolve loop exits
        * --------------------------------------------------------
        */

        let loop_end =
            self.chunk.code.len();

        self.chunk.patch_operand(
            source_end_jump,
            loop_end as u32,
        );

        for jump in
            next_item_jumps
        {
            self.chunk.patch_operand(
                jump,
                loop_start as u32,
            );
        }

        for jump in
            end_jumps
        {
            self.chunk.patch_operand(
                jump,
                loop_end as u32,
            );
        }

        /*
        * IteratorNext pushes Unit when exhausted.
        * Direct Range does not.
        */
        if iterator_slot.is_some() {
            self.chunk.emit(
                OpCode::Pop
            );
        }

        /*
        * --------------------------------------------------------
        * 10. Result
        * --------------------------------------------------------
        */

        self.chunk.emit_operand(
            OpCode::LoadLocal,
            result_slot as u32,
        );

        Ok(())
    }

    fn compile_pipeline_lambda(
        &mut self,
        lambda: &Expr,
        item_slot: u16,
    ) -> Result<()> {
        let ExprKind::Lambda(
            params,
            body,
        ) =
            &lambda.kind
        else {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline stage requires a lambda",
                    None,
                )
            );
        };

        if params.len() != 1 {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    "fused map/filter lambda must take exactly one argument",
                    None,
                )
            );
        }

        let Pattern::Ident(
            name
        ) =
            &params[0]
        else {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "fused map/filter parameter must be an identifier",
                    None,
                )
            );
        };

        if !Self::is_fusable_pipeline_expr(
            body
        ) {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline lambda contains unsupported control flow for fusion",
                    None,
                )
            );
        }

        let parent =
            self.scope.clone();

        let inline_scope =
            Scope::new(
                Some(parent.clone()),
                false,
            );

        inline_scope
            .borrow_mut()
            .locals
            .insert(
                name.clone(),
                item_slot,
            );

        self.scope =
            inline_scope;

        let result =
            self.compile_expr(
                body
            );

        self.scope =
            parent;

        result
    }

    fn try_compile_fused_collect(
        &mut self,
        callee: &Expr,
        args: &[CallArg],
    ) -> Result<bool> {
        if !args.is_empty() {
            return Ok(false);
        }

        let ExprKind::Field {
            object,
            name,
        } = &callee.kind
        else {
            return Ok(false);
        };

        if name != "collect" {
            return Ok(false);
        }

        let Some((
            source,
            stages,
        )) =
            self.extract_pipeline(object)?
        else {
            return Ok(false);
        };

        /*
        * Check the lambda BODY, not ExprKind::Lambda itself.
        */
        for stage in &stages {
            match stage {
                PipelineStage::Map(
                    lambda
                )
                |
                PipelineStage::Filter(
                    lambda
                ) => {
                    let ExprKind::Lambda(
                        _,
                        body,
                    ) = &lambda.kind
                    else {
                        return Ok(false);
                    };

                    if !Self::is_fusable_pipeline_expr(
                        body
                    ) {
                        return Ok(false);
                    }
                }

                PipelineStage::Skip(_)
                |
                PipelineStage::Take(_) => {}
            }
        }

        self.compile_fused_pipeline(
            &source,
            &stages,
        )?;

        Ok(true)
    }


}
