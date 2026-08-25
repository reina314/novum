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
    },
    runtime::Value,
};

use super::{
    Chunk,
    OpCode,
    UpvalueSpec,
    FunctionProto,
};

use std::{
    rc::Rc,
    cell::RefCell,
    collections::HashMap,
};

enum AssignmentTarget {
    Local(u16),
    Upvalue(u16),
    ImplicitLocal(u16),
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

    fn resolve_assignment(
        &mut self,
        name: &str,
    ) -> Result<AssignmentTarget> {
        if let Some(slot) =
            self.resolve_local(name)
        {
            return Ok(
                AssignmentTarget::Local(
                    slot
                )
            );
        }

        if let Some(slot) =
            self.resolve_upvalue(name)
        {
            return Ok(
                AssignmentTarget::Upvalue(
                    slot
                )
            );
        }

        let slot =
            self.declare_local(
                name.to_string()
            )?;

        Ok(
            AssignmentTarget::ImplicitLocal(
                slot
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

    fn add_upvalue(
        &mut self,
        name: &str,
        spec: UpvalueSpec,
    ) -> u16 {
        let mut scope =
            self.scope.borrow_mut();

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
        let parent =
            self.scope
                .borrow()
                .parent
                .clone()?;

        self.resolve_upvalue_from(
            parent,
            name,
        )
    }

    fn resolve_upvalue_from(
        &mut self,
        scope: ScopeRef,
        name: &str,
    ) -> Option<u16> {
        // Search every lexical scope inside the same
        // function first.

        if let Some(slot) =
            scope.borrow()
                .locals
                .get(name)
                .copied()
        {
            return Some(
                self.add_upvalue(
                    name,
                    UpvalueSpec::Local(slot),
                )
            );
        }

        if let Some(slot) =
            scope.borrow()
                .upvalues
                .get(name)
                .copied()
        {
            return Some(
                self.add_upvalue(
                    name,
                    UpvalueSpec::Parent(slot),
                )
            );
        }

        let parent =
            scope
                .borrow()
                .parent
                .clone()?;

        if scope
            .borrow()
            .function_boundary
        {
            return None;
        }

        self.resolve_upvalue_from(
            parent,
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

            ExprKind::Let {
                pattern,
                value,
                ..
            } => {
                match pattern {
                    Pattern::Ident(name) => {
                        self.compile_expr(value)?;

                        self.chunk.emit(
                            OpCode::Dup
                        );

                        let slot =
                            self.declare_local(
                                name.clone()
                            )?;

                        self.chunk.emit_operand(
                            OpCode::StoreLocal,
                            slot as u32,
                        );
                    }

                    _ => {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM currently supports only identifier bindings",
                                None,
                            )
                        );
                    }
                }
            }

            ExprKind::Assign {
                target,
                value,
            } => {
                let ExprKind::Ident(name) =
                    &target.kind
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently supports only identifier assignment",
                            None,
                        )
                    );
                };

                self.compile_expr(value)?;

                self.chunk.emit(
                    OpCode::Dup
                );

                match self.resolve_assignment(name)? {
                    AssignmentTarget::Local(slot) |
                    AssignmentTarget::ImplicitLocal(slot) => {
                        self.chunk.emit_operand(
                            OpCode::StoreLocal,
                            slot as u32,
                        );
                    }

                    AssignmentTarget::Upvalue(slot) => {
                        self.chunk.emit_operand(
                            OpCode::StoreUpvalue,
                            slot as u32,
                        );
                    }
                }
            }

            ExprKind::AssignOp {
                target,
                op,
                value,
            } => {
                let ExprKind::Ident(name) =
                    &target.kind
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "VM currently supports only identifier assignment",
                            None,
                        )
                    );
                };

                let variable =
                    if let Some(slot) =
                        self.resolve_local(name)
                    {
                        (false, slot)
                    } else if let Some(slot) =
                        self.resolve_upvalue(name)
                    {
                        (true, slot)
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
                    };

                if variable.0 {
                    self.chunk.emit_operand(
                        OpCode::LoadUpvalue,
                        variable.1 as u32,
                    );
                } else {
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        variable.1 as u32,
                    );
                }

                self.compile_expr(
                    value
                )?;

                self.compile_binop(
                    *op
                )?;

                self.chunk.emit(
                    OpCode::Dup
                );

                if variable.0 {
                    self.chunk.emit_operand(
                        OpCode::StoreUpvalue,
                        variable.1 as u32,
                    );
                } else {
                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        variable.1 as u32,
                    );
                }
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
                // Compile the callee first.
                self.compile_expr(
                    callee
                )?;

                // Compile arguments in source order.
                for arg in args {
                    // VM currently supports only positional arguments.
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
}