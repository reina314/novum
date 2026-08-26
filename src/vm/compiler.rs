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
    },
    runtime::Value,
};

use super::{
    Chunk,
    OpCode,
    UpvalueSpec,
    FunctionProto,
    encode_method_call,
};

use std::{
    rc::Rc,
    cell::RefCell,
    collections::HashMap,
};


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
    condition_target: usize,
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
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            scope: Scope::new(None, true,),
            next_local_slot: 0,
            loops: Vec::new(),
            loop_local_stack: Vec::new(),
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
            };

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
        }

        compiler.next_local_slot =
            params.len() as u16;

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
                let _ = name;

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    *object_slot as u32,
                );

                return Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "VM field access is not implemented yet",
                        None,
                    )
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
                let _ = object_slot;
                let _ = name;

                return Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "VM field assignment is not implemented yet",
                        None,
                    )
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
            chunk: Rc::new(chunk),
            upvalue_specs,
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
                        condition_target,

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
                        condition_target:
                            loop_start,

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

                let proto =
                    compiler.finish_function(
                        params.len() as u16
                    );

                let function =
                    Rc::new(proto);

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
                // receiver.method(...)
                if let ExprKind::Field {
                    object,
                    name,
                } = &callee.kind
                {
                    self.compile_expr(
                        object
                    )?;

                    for arg in args {
                        if arg.name.is_some() {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "VM currently does not support named arguments",
                                    None,
                                )
                            );
                        }

                        self.compile_expr(
                            &arg.value
                        )?;
                    }

                    let method_index =
                        self.chunk.add_constant(
                            Value::Str(
                                Rc::new(
                                    name.clone()
                                )
                            )
                        );

                    let argc =
                        args.len();

                    if argc >
                        u16::MAX as usize
                    {
                        return Err(
                            Error::new(
                                ErrorKind::Arity,
                                "too many method arguments",
                                None,
                            )
                        );
                    }

                    self.chunk.emit_operand(
                        OpCode::InvokeMethod,
                        encode_method_call(
                            method_index as u16,
                            argc as u16,
                        ),
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
                    if arg.name.is_some() {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM currently does not support named arguments",
                                None,
                            )
                        );
                    }

                    self.compile_expr(
                        &arg.value
                    )?;
                }

                self.chunk.emit_operand(
                    OpCode::Call,
                    args.len() as u32,
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

            Pattern::Enum { .. } => {
                return Err(
                    Error::new(
                        ErrorKind::Runtime,
                        "enum patterns are not implemented yet",
                        None,
                    )
                );
            }
        }

        Ok(failures)
    }

}