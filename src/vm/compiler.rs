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
    collections::HashMap
};

struct LoopContext {
    continue_target: usize,
    break_jumps: Vec<usize>,
}

#[derive(Clone)]
struct CompilerParent {
    locals: HashMap<String, u16>,
    upvalues: HashMap<String, u16>,
}

pub struct Compiler {
    chunk: Chunk,
    locals: HashMap<String, u16>,
    upvalues: HashMap<String, u16>,
    upvalue_specs: Vec<UpvalueSpec>,
    loops: Vec<LoopContext>,
    parent: Option<CompilerParent>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk:
                Chunk::default(),

            locals:
                HashMap::new(),

            upvalues:
                HashMap::new(),

            upvalue_specs:
                Vec::new(),

            loops:
                Vec::new(),

            parent:
                None,
        }
    }

    fn new_function(
        parent: &Compiler,
        params: &[Pattern],
    ) -> Result<Self> {
        let mut compiler =
            Self::new();

        compiler.parent =
            Some(
                CompilerParent {
                    locals:
                        parent.locals.clone(),

                    upvalues:
                        parent.upvalues.clone(),
                }
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

            compiler.locals.insert(
                name.clone(),
                index as u16,
            );
        }

        Ok(compiler)
    }

    fn add_local(
        &mut self,
        name: String,
    ) -> u16 {
        let slot =
            self.locals.len() as u16;

        self.locals.insert(
            name,
            slot,
        );

        slot
    }

    fn resolve_local(
        &self,
        name: &str,
    ) -> Option<u16> {
        self.locals
            .get(name)
            .copied()
    }
    
    fn resolve_upvalue(
        &mut self,
        name: &str,
    ) -> Option<u16> {
        if let Some(slot) =
            self.upvalues.get(name)
        {
            return Some(*slot);
        }

        let parent =
            self.parent.as_ref()?;

        if let Some(slot) =
            parent.locals.get(name)
        {
            let index =
                self.upvalue_specs.len()
                    as u16;

            self.upvalue_specs.push(
                UpvalueSpec::Local(
                    *slot
                )
            );

            self.upvalues.insert(
                name.to_string(),
                index,
            );

            return Some(index);
        }

        if let Some(slot) =
            parent.upvalues.get(name)
        {
            let index =
                self.upvalue_specs.len()
                    as u16;

            self.upvalue_specs.push(
                UpvalueSpec::Parent(
                    *slot
                )
            );

            self.upvalues.insert(
                name.to_string(),
                index,
            );

            return Some(index);
        }

        None
    }

    pub fn finish(self) -> Chunk {
        let mut chunk =
            self.chunk;

        chunk.local_count =
            self.locals.len();

        chunk
    }

    fn finish_function(
        self,
        arity: u16,
    ) -> FunctionProto {
        let mut chunk =
            self.chunk;

        chunk.local_count =
            self.locals.len();

        FunctionProto {
            arity,
            chunk: Rc::new(chunk),
            upvalue_specs:
                self.upvalue_specs,
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
                self.compile_expr(
                    value
                )?;

                match pattern {
                    Pattern::Ident(name) => {
                        let slot =
                            self.add_local(
                                name.clone()
                            );

                        self.chunk.emit_operand(
                            OpCode::StoreLocal,
                            slot as u32,
                        );

                        self.chunk.emit(
                            OpCode::Unit
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

                self.compile_expr(
                    value
                )?;

                self.chunk.emit(
                    OpCode::Dup
                );

                if let Some(slot) =
                    self.resolve_local(name)
                {
                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );
                } else if let Some(slot) =
                    self.resolve_upvalue(name)
                {
                    self.chunk.emit_operand(
                        OpCode::StoreUpvalue,
                        slot as u32,
                    );
                } else {
                    let slot =
                        self.add_local(
                            name.clone()
                        );

                    self.chunk.emit_operand(
                        OpCode::StoreLocal,
                        slot as u32,
                    );
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
                    self.locals.get(name)
                {
                    self.chunk.emit_operand(
                        OpCode::LoadLocal,
                        *slot as u32,
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
                let loop_start =
                    self.chunk.code.len();

                self.loops.push(
                    LoopContext {
                        continue_target:
                            loop_start,
                        break_jumps:
                            Vec::new(),
                    }
                );

                self.compile_expr(
                    cond
                )?;

                let exit =
                    self.chunk.emit_operand(
                        OpCode::JumpIfFalse,
                        0,
                    );

                self.compile_expr(
                    body
                )?;

                self.chunk.emit(
                    OpCode::Pop
                );

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
                    self.loops
                        .pop()
                        .unwrap();

                for jump in
                    context.break_jumps
                {
                    self.chunk.patch_operand(
                        jump,
                        loop_end as u32,
                    );
                }

                self.chunk.emit(
                    OpCode::Unit
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
                    self.loops.last()
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Control,
                            "continue outside loop",
                            None,
                        )
                    );
                };

                self.chunk.emit_operand(
                    OpCode::Jump,
                    loop_context
                        .continue_target
                        as u32,
                );
            }

            ExprKind::Lambda(
                params,
                body,
            ) => {
                let mut compiler =
                    Compiler::new_function(
                        self,
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