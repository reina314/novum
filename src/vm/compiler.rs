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
        Visibility,
    },
    runtime::{
        Value,
        StructType,
        UpvalueSpec,
        FunctionProto,
        FunctionRef,
        ModulePath,
    },
    stdlib::{
        encode_class_counts,
        is_self_pattern,
    }
};

use super::{
    Chunk,
    RangeLoop,
    OpCode,
    PipelineExpr,
    PipelineStage,
    PipelineSource,
    PipelineProgram,
    PipelinePlan,
    IntPipelineExpr,
    IntPipelinePredicate,
    IntPipelineStage,
};

use std::{
    rc::Rc,
    cell::RefCell,
    collections::HashMap,
};

enum PipelineStageAst {
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
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
    continue_target: Option<usize>,
    cleanup_target: Option<usize>,
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

    fn emit_loop_cleanup(
        &mut self,
        loop_index: usize,
    ) {
        let slots =
            self.loops[loop_index]
                .local_slots
                .clone();

        for slot in slots {
            self.chunk.emit_operand(
                OpCode::ResetLocal,
                slot as u32,
            );
        }
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

    fn begin_loop(
        &mut self,
    ) -> usize {
        let loop_index =
            self.loops.len();

        self.loops.push(
            LoopContext {
                break_jumps:
                    Vec::new(),

                continue_jumps:
                    Vec::new(),

                continue_target:
                    None,

                cleanup_target:
                    None,

                local_slots:
                    Vec::new(),
            }
        );

        self.loop_local_stack.push(
            Vec::new()
        );

        loop_index
    }

    fn set_loop_cleanup(
        &mut self,
        loop_index: usize,
        cleanup_target: usize,
    ) {
        self.loops[loop_index]
            .cleanup_target =
            Some(
                cleanup_target
            );
    }

    fn set_continue_target(
        &mut self,
        loop_index: usize,
        target: usize,
    ) {
        self.loops[loop_index]
            .continue_target =
            Some(target);

        let jumps =
            std::mem::take(
                &mut self.loops[loop_index]
                    .continue_jumps
            );

        for jump in jumps {
            self.chunk.patch_operand(
                jump,
                target as u32,
            );
        }
    }

    fn finish_loop(
        &mut self,
        loop_index: usize,
        loop_end: usize,
    ) {
        /*
        * Patch all break jumps.
        */
        let break_jumps =
            std::mem::take(
                &mut self.loops[loop_index]
                    .break_jumps
            );

        for jump in break_jumps {
            self.chunk.patch_operand(
                jump,
                loop_end as u32,
            );
        }

        /*
        * Recover loop-local slots.
        */
        let local_slots =
            self.loop_local_stack
                .pop()
                .unwrap_or_default();

        self.loops[loop_index]
            .local_slots =
            local_slots;

        /*
        * Remove the loop context.
        */
        self.loops.pop();
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
            Vec<PipelineStageAst>,
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
                        PipelineStageAst::Map(
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
                        PipelineStageAst::Filter(
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
                        PipelineStageAst::Skip(
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
                        PipelineStageAst::Take(
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

    fn pipeline_lambda_is_fusable(
        lambda: &Expr,
    ) -> bool {
        let ExprKind::Lambda(
            params,
            body,
        ) = &lambda.kind
        else {
            return false;
        };

        if params.len() != 1 {
            return false;
        }

        Self::pipeline_expr_is_fusable(
            body
        )
    }

    fn pipeline_expr_is_fusable(
        expr: &Expr,
    ) -> bool {
        match &expr.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Ident(_)
            | ExprKind::Null
            | ExprKind::Unit => true,

            ExprKind::Neg(
                expr
            )
            |
            ExprKind::Not(
                expr
            ) => {
                Self::pipeline_expr_is_fusable(
                    expr
                )
            }

            ExprKind::Binary(
                _,
                left,
                right,
            ) => {
                Self::pipeline_expr_is_fusable(
                    left
                )
                &&
                Self::pipeline_expr_is_fusable(
                    right
                )
            }

            /*
            * These require the general VM execution path.
            * Do not put them into the fused pipeline function.
            */
            ExprKind::Lambda(..)
            | ExprKind::Call(..)
            | ExprKind::Field { .. }
            | ExprKind::Index(..)
            | ExprKind::Tuple(..)
            | ExprKind::TupleIndex { .. }
            | ExprKind::List(..)
            | ExprKind::Dict(..)
            | ExprKind::Range { .. }
            | ExprKind::Let { .. }
            | ExprKind::Assign { .. }
            | ExprKind::AssignOp { .. }
            | ExprKind::If(..)
            | ExprKind::While(..)
            | ExprKind::For { .. }
            | ExprKind::Match { .. }
            | ExprKind::Break
            | ExprKind::Continue
            | ExprKind::Return(_)
            | ExprKind::Try(_)
            | ExprKind::StructDecl { .. }
            | ExprKind::ClassDecl { .. }
            | ExprKind::EnumDecl(_)
            | ExprKind::Import { .. }
            | ExprKind::Drop(_)
            | ExprKind::Block(_) => false,
        }
    }

    fn lower_pipeline_expr(
        &mut self,
        expr: &Expr,
        input_name: &str,
    ) -> Result<PipelineExpr> {
        match &expr.kind {
            ExprKind::Int(value) =>
                Ok(
                    PipelineExpr::Int(
                        *value
                    )
                ),

            ExprKind::Float(value) =>
                Ok(
                    PipelineExpr::Float(
                        *value
                    )
                ),

            ExprKind::Bool(value) =>
                Ok(
                    PipelineExpr::Bool(
                        *value
                    )
                ),

            ExprKind::Str(value) =>
                Ok(
                    PipelineExpr::Str(
                        value.clone()
                    )
                ),

            ExprKind::Ident(name) => {
                if name == input_name {
                    return Ok(
                        PipelineExpr::Input
                    );
                }

                let slot = 
                    self.resolve_upvalue(name)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "{} is undefined",
                                name
                            ),
                            None,
                        )
                    })?;

                Ok(
                    PipelineExpr::Capture(
                        slot
                    )
                )
            }

            ExprKind::Neg(expr) => {
                Ok(
                    PipelineExpr::Neg(
                        Box::new(
                            self.lower_pipeline_expr(
                                expr,
                                input_name,
                            )?
                        )
                    )
                )
            }

            ExprKind::Not(expr) => {
                Ok(
                    PipelineExpr::Not(
                        Box::new(
                            self.lower_pipeline_expr(
                                expr,
                                input_name,
                            )?
                        )
                    )
                )
            }

            ExprKind::Binary(
                op,
                left,
                right,
            ) => {
                let left =
                    Box::new(
                        self.lower_pipeline_expr(
                            left,
                            input_name,
                        )?
                    );

                let right =
                    Box::new(
                        self.lower_pipeline_expr(
                            right,
                            input_name,
                        )?
                    );

                match op {
                    BinOp::Add =>
                        Ok(
                            PipelineExpr::Add(
                                left,
                                right,
                            )
                        ),

                    BinOp::Sub =>
                        Ok(
                            PipelineExpr::Sub(
                                left,
                                right,
                            )
                        ),

                    BinOp::Mul =>
                        Ok(
                            PipelineExpr::Mul(
                                left,
                                right,
                            )
                        ),

                    BinOp::Div =>
                        Ok(
                            PipelineExpr::Div(
                                left,
                                right,
                            )
                        ),

                    BinOp::Mod =>
                        Ok(
                            PipelineExpr::Mod(
                                left,
                                right,
                            )
                        ),

                    BinOp::Pow =>
                        Ok(
                            PipelineExpr::Pow(
                                left,
                                right,
                            )
                        ),

                    BinOp::Eq =>
                        Ok(
                            PipelineExpr::Eq(
                                left,
                                right,
                            )
                        ),

                    BinOp::Neq =>
                        Ok(
                            PipelineExpr::Neq(
                                left,
                                right,
                            )
                        ),

                    BinOp::Lt =>
                        Ok(
                            PipelineExpr::Lt(
                                left,
                                right,
                            )
                        ),

                    BinOp::Leq =>
                        Ok(
                            PipelineExpr::Leq(
                                left,
                                right,
                            )
                        ),

                    BinOp::Gt =>
                        Ok(
                            PipelineExpr::Gt(
                                left,
                                right,
                            )
                        ),

                    BinOp::Geq =>
                        Ok(
                            PipelineExpr::Geq(
                                left,
                                right,
                            )
                        ),

                    _ =>
                        Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "operator is not supported in fused pipeline",
                                None,
                            )
                        ),
                }
            }

            _ =>
                Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "expression is not supported in fused pipeline",
                        None,
                    )
                ),
        }
    }

    fn lower_pipeline_lambda(
        &mut self,
        lambda: &Expr,
    ) -> Result<(
        PipelineExpr,
        Vec<UpvalueSpec>,
    )> {
        let ExprKind::Lambda(
            params,
            body,
        ) =
            &lambda.kind
        else {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline stage requires lambda",
                    None,
                )
            );
        };

        if params.len() != 1 {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    "pipeline lambda must take exactly one argument",
                    None,
                )
            );
        }

        let Pattern::Ident(
            parameter
        ) =
            &params[0]
        else {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline lambda parameter must be an identifier",
                    None,
                )
            );
        };

        /*
        * Create a compiler-only function scope.
        *
        * This scope is not emitted as a runtime function.
        * It exists so normal lexical/upvalue resolution can
        * be reused for the pipeline lambda.
        */
        let parent =
            self.scope.clone();

        let lambda_scope =
            Scope::new(
                Some(parent),
                true,
            );

        /*
        * The pipeline input is local slot 0 inside the
        * synthetic lambda scope.
        */
        lambda_scope
            .borrow_mut()
            .locals
            .insert(
                parameter.clone(),
                0,
            );

        let previous_scope =
            std::mem::replace(
                &mut self.scope,
                lambda_scope.clone(),
            );

        let result =
            self.lower_pipeline_expr(
                body,
                parameter,
            );

        /*
        * Capture specification belongs to this
        * synthetic lambda scope.
        */
        let captures =
            lambda_scope
                .borrow()
                .upvalue_specs
                .clone();

        self.scope =
            previous_scope;

        Ok((
            result?,
            captures,
        ))
    }

    fn lower_int_pipeline_expr(
        expr: &PipelineExpr,
    ) -> Option<IntPipelineExpr> {
        match expr {
            PipelineExpr::Input =>
                Some(
                    IntPipelineExpr::Input
                ),

            PipelineExpr::Int(value) =>
                Some(
                    IntPipelineExpr::Const(
                        *value
                    )
                ),

            PipelineExpr::Capture(slot) =>
                Some(
                    IntPipelineExpr::Capture(
                        *slot
                    )
                ),

            PipelineExpr::Add(
                left,
                right,
            ) => Some(
                IntPipelineExpr::Add(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            left
                        )?
                    ),
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            right
                        )?
                    ),
                )
            ),

            PipelineExpr::Sub(
                left,
                right,
            ) => Some(
                IntPipelineExpr::Sub(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            left
                        )?
                    ),
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            right
                        )?
                    ),
                )
            ),

            PipelineExpr::Mul(
                left,
                right,
            ) => Some(
                IntPipelineExpr::Mul(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            left
                        )?
                    ),
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            right
                        )?
                    ),
                )
            ),

            PipelineExpr::Div(
                left,
                right,
            ) => Some(
                IntPipelineExpr::Div(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            left
                        )?
                    ),
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            right
                        )?
                    ),
                )
            ),

            PipelineExpr::Mod(
                left,
                right,
            ) => Some(
                IntPipelineExpr::Mod(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            left
                        )?
                    ),
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            right
                        )?
                    ),
                )
            ),

            PipelineExpr::Neg(
                expr
            ) => Some(
                IntPipelineExpr::Neg(
                    Box::new(
                        Self::lower_int_pipeline_expr(
                            expr
                        )?
                    )
                )
            ),

            _ =>
                None,
        }
    }

    fn lower_int_predicate(
        expr: &PipelineExpr,
    ) -> Option<IntPipelinePredicate> {
        match expr {
            PipelineExpr::Eq(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Eq(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            PipelineExpr::Neq(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Neq(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            PipelineExpr::Lt(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Lt(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            PipelineExpr::Leq(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Leq(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            PipelineExpr::Gt(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Gt(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            PipelineExpr::Geq(
                left,
                right,
            ) =>
                Some(
                    IntPipelinePredicate::Geq(
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                left
                            )?
                        ),
                        Box::new(
                            Self::lower_int_pipeline_expr(
                                right
                            )?
                        ),
                    )
                ),

            _ =>
                None,
        }
    }

    fn lower_int_pipeline_plan(
        &self,
        stages: &[PipelineStage],
    ) -> PipelinePlan {
        let mut result =
            Vec::with_capacity(
                stages.len()
            );

        for stage in stages {
            match stage {
                PipelineStage::Map {
                    expr,
                    ..
                } => {
                    let Some(
                        expr
                    ) =
                        Self::lower_int_pipeline_expr(
                            expr
                        )
                    else {
                        return PipelinePlan::Generic;
                    };

                    result.push(
                        IntPipelineStage::Map(
                            expr
                        )
                    );
                }

                PipelineStage::Filter {
                    expr,
                    ..
                } => {
                    let Some(
                        predicate
                    ) =
                        Self::lower_int_predicate(
                            expr
                        )
                    else {
                        return PipelinePlan::Generic;
                    };

                    result.push(
                        IntPipelineStage::Filter(
                            predicate
                        )
                    );
                }

                PipelineStage::Skip {
                    count
                } => {
                    result.push(
                        IntPipelineStage::Skip(
                            *count
                        )
                    );
                }

                PipelineStage::Take {
                    count
                } => {
                    result.push(
                        IntPipelineStage::Take(
                            *count
                        )
                    );
                }
            }
        }

        PipelinePlan::IntRange {
            stages: result
        }
    }

    fn range_for_parts(
        expr: &Expr,
    ) -> Option<(
        &Expr,
        &Expr,
        bool,
    )> {
        match &expr.kind {
            ExprKind::Range {
                start: Some(start),
                end: Some(end),
                inclusive,
            } => {
                Some((
                    start,
                    end,
                    *inclusive,
                ))
            }

            _ =>
                None,
        }
    }

    fn is_module_scope(&self) -> bool {
        self.scope
            .borrow()
            .parent
            .is_none()
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
                visibility,
                name,
                fields,
                methods,
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

                if *visibility ==
                    Visibility::Public
                {
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
                        OpCode::Export
                    );
                }

                self.chunk.emit(
                    OpCode::Pop
                );

                self.chunk.emit(
                    OpCode::Unit
                );
            }

            ExprKind::ClassDecl {
                visibility,
                name,
                fields,
                methods,
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

                if *visibility ==
                    Visibility::Public
                {
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
                        OpCode::Export
                    );
                }

                self.chunk.emit(
                    OpCode::Pop
                );

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
                visibility,
                pattern,
                value,
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

                match visibility {
                    Visibility::Private => {}

                    Visibility::Public => {
                        if !self.is_module_scope() {
                            return Err(
                                Error::new(
                                    ErrorKind::Name,
                                    "pub declarations are only allowed at module scope",
                                    None,
                                )
                            );
                        }
                        
                        let Pattern::Ident(name) =
                            pattern
                        else {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "pub let currently requires an identifier pattern",
                                    None,
                                )
                            );
                        };

                        let slot =
                            self.resolve_local(name)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Name,
                                        format!(
                                            "exported binding '{}' was not declared",
                                            name
                                        ),
                                        None,
                                    )
                                })?;

                        self.chunk.emit_operand(
                            OpCode::LoadLocal,
                            slot as u32,
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
                            OpCode::Export
                        );
                    }
                }

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
                condition,
                body,
            ) => {
                /*
                * --------------------------------------------------------
                * Result slot
                * --------------------------------------------------------
                *
                * A while-expression evaluates to Unit if the loop
                * executes zero times, otherwise to the last body value.
                */
                let result_slot =
                    self.allocate_temp_local();

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

                /*
                * --------------------------------------------------------
                * Loop context
                * --------------------------------------------------------
                */
                let loop_index =
                    self.begin_loop();

                /*
                * --------------------------------------------------------
                * Condition entry
                * --------------------------------------------------------
                */
                let condition_start =
                    self.chunk.code.len();

                /*
                * `continue` jumps directly to the condition.
                */
                self.set_continue_target(
                    loop_index,
                    condition_start,
                );

                /*
                * --------------------------------------------------------
                * Condition
                * --------------------------------------------------------
                */
                self.compile_expr(
                    condition
                )?;

                let exit_jump =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                /*
                * JumpIfFalse already consumes the Bool.
                * DO NOT emit Pop here.
                */

                /*
                * --------------------------------------------------------
                * Body
                * --------------------------------------------------------
                */
                self.enter_scope();

                self.compile_expr(
                    body
                )?;

                /*
                * Preserve the value produced by this iteration
                * as the result of the entire while-expression.
                *
                * Stack:
                *
                *     [body_value]
                *
                * Dup:
                *
                *     [body_value, body_value]
                *
                * StoreLocal consumes one:
                *
                *     [body_value]
                */
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

                /*
                * Discard the remaining body value before the next
                * iteration.
                */
                self.chunk.emit(
                    OpCode::Pop
                );

                /*
                * --------------------------------------------------------
                * End body scope
                * --------------------------------------------------------
                */
                self.exit_scope();

                /*
                * --------------------------------------------------------
                * Cleanup
                * --------------------------------------------------------
                */
                let cleanup_target =
                    self.chunk.code.len();

                self.set_loop_cleanup(
                    loop_index,
                    cleanup_target,
                );

                self.emit_loop_cleanup(
                    loop_index
                );

                /*
                * --------------------------------------------------------
                * Next iteration
                * --------------------------------------------------------
                */
                self.chunk.emit_operand(
                    OpCode::Jump,
                    condition_start as u32,
                );

                /*
                * --------------------------------------------------------
                * Loop end
                * --------------------------------------------------------
                */
                let loop_end =
                    self.chunk.code.len();

                self.chunk.patch_operand(
                    exit_jump,
                    loop_end as u32,
                );

                /*
                * JumpIfFalse already consumed the false condition.
                * DO NOT emit Pop here.
                */

                self.finish_loop(
                    loop_index,
                    loop_end,
                );

                /*
                * --------------------------------------------------------
                * Final result
                * --------------------------------------------------------
                */
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
                if let Some((
                    start,
                    end,
                    inclusive,
                )) = Self::range_for_parts(
                    iterable
                ) {
                    self.compile_range_for(
                        pattern,
                        start,
                        end,
                        inclusive,
                        body,
                    )?;
                } else {
                    self.compile_generic_for(
                        pattern,
                        iterable,
                        body,
                    )?;
                }
            }

            ExprKind::Break => {
                let jump =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                let Some(loop_index) =
                    self.loops.len()
                        .checked_sub(1)
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "break outside loop",
                            None,
                        )
                    );
                };

                self.loops[loop_index]
                    .break_jumps
                    .push(
                        jump
                    );
            }

            ExprKind::Continue => {
                let jump =
                    self.chunk.emit_operand(
                        OpCode::Jump,
                        0,
                    );

                let Some(loop_index) =
                    self.loops.len()
                        .checked_sub(1)
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "continue outside loop",
                            None,
                        )
                    );
                };

                if let Some(target) =
                    self.loops[loop_index]
                        .continue_target
                {
                    self.chunk.patch_operand(
                        jump,
                        target as u32,
                    );
                } else {
                    self.loops[loop_index]
                        .continue_jumps
                        .push(
                            jump
                        );
                }
            }

            ExprKind::Return(
                value
            ) => {
                match value {
                    Some(value) => {
                        self.compile_expr(
                            value
                        )?;

                        self.chunk.emit(
                            OpCode::Return
                        );
                    }

                    None => {
                        self.chunk.emit(
                            OpCode::Unit
                        );

                        self.chunk.emit(
                            OpCode::Return
                        );
                    }
                }
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

            ExprKind::Import {
                path,
                alias,
            } => {
                if path.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Import,
                            "empty module path",
                            None,
                        )
                    );
                }

                /*
                * Phase 3:
                *
                * import foo
                * import foo as bar
                * import a.b.c as x
                *
                * Nested import without alias is deferred.
                */
                if alias.is_none()
                    && path.len() != 1
                {
                    return Err(
                        Error::new(
                            ErrorKind::Import,
                            "nested imports without alias are not supported yet",
                            None,
                        )
                    );
                }

                let binding_name =
                    match alias {
                        Some(alias) => {
                            alias.clone()
                        }

                        None => {
                            path[0].clone()
                        }
                    };

                let slot =
                    self.declare_local(
                        binding_name
                    )?;

                let module_path =
                    ModulePath::new(
                        path.clone()
                    );

                let module_ref =
                    self.chunk.add_module_ref(
                        module_path
                    );

                self.chunk.emit_operand(
                    OpCode::LoadModule,
                    module_ref,
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

    fn compile_generic_for(
        &mut self,
        pattern: &Pattern,
        iterable: &Expr,
        body: &Expr,
    ) -> Result<()> {
        /*
        * --------------------------------------------------------
        * Result slot
        * --------------------------------------------------------
        */
        let result_slot =
            self.allocate_temp_local();

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

        /*
        * --------------------------------------------------------
        * Iterator
        * --------------------------------------------------------
        */
        self.compile_expr(
            iterable
        )?;

        self.chunk.emit(
            OpCode::IteratorFrom
        );

        let iterator_slot =
            self.allocate_temp_local();

        self.chunk.emit_operand(
            OpCode::StoreLocal,
            iterator_slot as u32,
        );

        self.chunk.emit(
            OpCode::Pop
        );

        /*
        * --------------------------------------------------------
        * Loop
        * --------------------------------------------------------
        */
        let loop_index =
            self.begin_loop();

        let loop_start =
            self.chunk.code.len();

        /*
        * IteratorNext:
        *
        *     [item, true]
        *
        * or
        *
        *     [Unit, false]
        */
        self.chunk.emit_operand(
            OpCode::LoadLocal,
            iterator_slot as u32,
        );

        self.chunk.emit(
            OpCode::IteratorNext
        );

        let exit_jump =
            self.chunk.emit_operand(
                OpCode::JumpIfFalse,
                0,
            );

        /*
        * --------------------------------------------------------
        * Bind current iteration value
        * --------------------------------------------------------
        */
        self.enter_scope();

        let value_slot =
            self.allocate_temp_local();

        self.chunk.emit_operand(
            OpCode::StoreLocal,
            value_slot as u32,
        );

        self.chunk.emit(
            OpCode::Pop
        );

        self.compile_pattern(
            value_slot,
            pattern,
        )?;

        /*
        * --------------------------------------------------------
        * Body
        * --------------------------------------------------------
        */
        self.compile_expr(
            body
        )?;

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

        /*
        * Discard body result before leaving the iteration.
        */
        self.chunk.emit(
            OpCode::Pop
        );

        /*
        * --------------------------------------------------------
        * Cleanup
        * --------------------------------------------------------
        */
        self.exit_scope();

        let cleanup_target =
            self.chunk.code.len();

        self.set_loop_cleanup(
            loop_index,
            cleanup_target,
        );

        /*
        * For-loop continue enters cleanup first.
        */
        self.set_continue_target(
            loop_index,
            cleanup_target,
        );

        self.emit_loop_cleanup(
            loop_index
        );

        /*
        * --------------------------------------------------------
        * Next iteration
        * --------------------------------------------------------
        */
        self.chunk.emit_operand(
            OpCode::Jump,
            loop_start as u32,
        );

        /*
        * --------------------------------------------------------
        * Loop end
        * --------------------------------------------------------
        */
        let loop_end =
            self.chunk.code.len();

        self.chunk.patch_operand(
            exit_jump,
            loop_end as u32,
        );

        /*
        * IteratorNext's false result:
        *
        *     [Unit, false]
        *
        * JumpIfFalse consumes false but Unit remains.
        */
        self.chunk.emit(
            OpCode::Pop
        );

        self.finish_loop(
            loop_index,
            loop_end,
        );

        /*
        * Result of the whole for-expression.
        */
        self.chunk.emit_operand(
            OpCode::LoadLocal,
            result_slot as u32,
        );

        Ok(())
    }

    fn compile_range_for(
        &mut self,
        pattern: &Pattern,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
        body: &Expr,
    ) -> Result<()> {
        /*
        * A for-expression evaluates to Unit when it executes zero
        * iterations, otherwise to the last evaluated body value.
        */
        let result_slot =
            self.allocate_temp_local();

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

        /*
        * Evaluate range bounds exactly once.
        *
        * RangeInit consumes:
        *
        *     [start, end]
        *
        * and initializes the runtime RangeCursor.
        */
        self.compile_expr(
            start
        )?;

        self.compile_expr(
            end
        )?;

        let loop_index =
            self.begin_loop();

        /*
        * The iteration binding lives inside its own scope so that
        * iteration-local variables can be reset at the cleanup point.
        */
        self.enter_scope();

        let value_slot =
            match pattern {
                Pattern::Ident(name) => {
                    self.declare_local(
                        name.clone()
                    )?
                }

                Pattern::Wildcard => {
                    self.allocate_temp_local()
                }

                _ => {
                    self.exit_scope();

                    self.loop_local_stack
                        .pop();

                    self.loops.pop();

                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently supports only identifier and wildcard for range-for patterns",
                            None,
                        )
                    );
                }
            };

        /*
        * Register compile-time metadata.
        *
        * The mutable range state itself is stored in the frame's
        * range_cursors and is initialized by RangeInit.
        */
        let range_loop_index =
            self.chunk.add_range_loop(
                RangeLoop {
                    value_slot,
                    inclusive,
                    exit_ip: 0,
                }
            );

        /*
        * Initialize the runtime cursor once, before entering
        * the loop.
        */
        self.chunk.emit_operand(
            OpCode::RangeInit,
            range_loop_index,
        );

        /*
        * RangeNext is the loop header. `continue` for a range-for
        * must eventually land at the cleanup section immediately
        * before this instruction.
        */
        let loop_start =
            self.chunk.code.len();

        /*
        * RangeNext writes the current range value directly into
        * value_slot. When exhausted, it redirects IP to exit_ip.
        */
        self.chunk.emit_operand(
            OpCode::RangeNext,
            range_loop_index,
        );

        /*
        * The body is compiled inside the iteration scope.
        */
        self.compile_expr(
            body
        )?;

        /*
        * Save the last body value as the result of the whole
        * for-expression.
        */
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

        /*
        * The iteration-local scope ends here.
        */
        self.exit_scope();

        /*
        * Continue jumps enter cleanup first.
        */
        let cleanup_target =
            self.chunk.code.len();

        self.set_loop_cleanup(
            loop_index,
            cleanup_target,
        );

        self.set_continue_target(
            loop_index,
            cleanup_target,
        );

        /*
        * Reset all locals introduced in the iteration scope.
        */
        self.emit_loop_cleanup(
            loop_index
        );

        /*
        * Next iteration.
        */
        self.chunk.emit_operand(
            OpCode::Jump,
            loop_start as u32,
        );

        /*
        * The instruction immediately following the back-edge is
        * the loop exit destination for RangeNext.
        */
        let loop_end =
            self.chunk.code.len();

        self.chunk.range_loops[
            range_loop_index as usize
        ].exit_ip =
            loop_end as u32;

        /*
        * Patch and destroy the loop context.
        */
        self.finish_loop(
            loop_index,
            loop_end,
        );

        /*
        * Return the final body value, or Unit for an empty range.
        */
        self.chunk.emit_operand(
            OpCode::LoadLocal,
            result_slot as u32,
        );

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

    fn compile_pipeline_program(
        &mut self,
        source: &Expr,
        stages: &[PipelineStageAst],
    ) -> Result<PipelineProgram> {
        let source =
            match &source.kind {
                ExprKind::Range {
                    start: Some(start),
                    end: Some(end),
                    inclusive,
                } => {
                    let ExprKind::Int(start) =
                        &start.kind
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "fused pipeline requires integer range start",
                                None,
                            )
                        );
                    };

                    let ExprKind::Int(end) =
                        &end.kind
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "fused pipeline requires integer range end",
                                None,
                            )
                        );
                    };

                    PipelineSource::Range {
                        start: *start,
                        end: *end,
                        inclusive: *inclusive,
                    }
                }

                _ => {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "unsupported fused pipeline source",
                            None,
                        )
                    );
                }
            };

        let mut compiled =
            Vec::with_capacity(
                stages.len()
            );

        for stage in stages {
            match stage {
                PipelineStageAst::Map(
                    lambda
                ) => {
                    let (
                        expr,
                        captures,
                    ) =
                        self.lower_pipeline_lambda(
                            lambda
                        )?;

                    compiled.push(
                        PipelineStage::Map {
                            expr,
                            captures,
                        }
                    );
                }

                PipelineStageAst::Filter(
                    lambda
                ) => {
                    let (
                        expr,
                        captures,
                    ) =
                        self.lower_pipeline_lambda(
                            lambda
                        )?;

                    compiled.push(
                        PipelineStage::Filter {
                            expr,
                            captures,
                        }
                    );
                }

                PipelineStageAst::Skip(
                    count
                ) => {
                    compiled.push(
                        PipelineStage::Skip {
                            count: *count,
                        }
                    );
                }

                PipelineStageAst::Take(
                    count
                ) => {
                    compiled.push(
                        PipelineStage::Take {
                            count: *count,
                        }
                    );
                }
            }
        }

        let plan =
            self.lower_int_pipeline_plan(
                &compiled
            );

        Ok(
            PipelineProgram {
                source,
                stages: compiled,
                plan,
            }
        )
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
        } =
            &callee.kind
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
            self.extract_pipeline(
                object
            )?
        else {
            return Ok(false);
        };

        /*
        * Current fused executor supports only:
        *
        *     map(|x| <simple expression>)
        *     filter(|x| <simple expression>)
        *
        * If any stage is outside that subset, do not fuse.
        * Falling back to the normal iterator implementation
        * preserves program semantics.
        */
        for stage in &stages {
            match stage {
                PipelineStageAst::Map(
                    lambda
                )
                |
                PipelineStageAst::Filter(
                    lambda
                ) => {
                    if !Self::pipeline_lambda_is_fusable(
                        lambda
                    ) {
                        return Ok(false);
                    }
                }

                PipelineStageAst::Skip(_)
                |
                PipelineStageAst::Take(_) => {}
            }
        }

        let pipeline =
            self.compile_pipeline_program(
                &source,
                &stages,
            )?;

        let pipeline_index =
            self.chunk.add_pipeline(
                pipeline
            );

        self.chunk.emit_operand(
            OpCode::FusedPipeline,
            pipeline_index,
        );

        Ok(true)
    }

}
