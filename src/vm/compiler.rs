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

pub struct Compiler {
    chunk: Chunk,
    locals: HashMap<String, u16>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            locals: HashMap::new(),
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

                if let Some(else_branch) =
                    else_branch
                {
                    self.compile_expr(
                        else_branch
                    )?;
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
            }

            ExprKind::Lambda(
                params,
                body,
            ) => {
                let mut compiler =
                    Compiler::new();

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