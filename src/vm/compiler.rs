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
    VmFunction,
};

use std::{
    rc::Rc,
    collections::HashMap
};

struct LoopContext {
    continue_target: usize,
    break_jumps: Vec<usize>,
}

pub struct Compiler {
    chunk: Chunk,
    locals: HashMap<String, u16>,
    loops: Vec<LoopContext>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            locals: HashMap::new(),
            loops: Vec::new(),
        }
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

    fn find_local(
        &self,
        name: &str,
    ) -> Option<u16> {
        self.locals.get(name).copied()
    }

    pub fn finish(self) -> Chunk {
        self.chunk
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

    pub fn new_function(
        params: &[Pattern],
    ) -> Result<Self> {
        let mut compiler =
            Self::new();

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
                match &target.kind {
                    ExprKind::Ident(name) => {
                        self.compile_expr(
                            value
                        )?;

                        self.chunk.emit(
                            OpCode::Dup
                        );

                        let slot =
                            match self.find_local(
                                name
                            ) {
                                Some(slot) =>
                                    slot,

                                None =>
                                    self.add_local(
                                        name.clone()
                                    ),
                            };

                        self.chunk.emit_operand(
                            OpCode::StoreLocal,
                            slot as u32,
                        );
                    }

                    _ => {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM currently supports only identifier assignment",
                                None,
                            )
                        );
                    }
                }
            }

            ExprKind::AssignOp {
                target,
                op,
                value,
            } => {
                match &target.kind {
                    ExprKind::Ident(name) => {
                        let slot =
                            self.find_local(
                                name
                            )
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

                        self.chunk.emit_operand(
                            OpCode::LoadLocal,
                            slot as u32,
                        );

                        self.compile_expr(
                            value
                        )?;

                        self.compile_binop(
                            *op
                        )?;

                        self.chunk.emit(
                            OpCode::Dup
                        );

                        self.chunk.emit_operand(
                            OpCode::StoreLocal,
                            slot as u32,
                        );
                    }

                    _ => {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM currently supports only identifier assignment",
                                None,
                            )
                        );
                    }
                }
            }

            ExprKind::Ident(name) => {
                let slot =
                    self.find_local(
                        name
                    )
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

                self.chunk.emit_operand(
                    OpCode::LoadLocal,
                    slot as u32,
                );
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
                        params
                    )?;

                compiler.compile_expr(
                    body
                )?;

                compiler.chunk.emit(
                    OpCode::Return
                );

                let chunk =
                    Rc::new(
                        compiler.finish()
                    );

                let function =
                    VmFunction {
                        arity:
                            params.len() as u16,
                        chunk,
                    };

                let index =
                    self.chunk.add_constant(
                        Value::VmFunction(
                            Rc::new(function)
                        )
                    );

                self.chunk.emit_operand(
                    OpCode::Constant,
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