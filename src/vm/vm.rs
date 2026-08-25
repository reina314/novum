use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    runtime::Value,
};

use super::{
    Chunk,
    OpCode,
    CallFrame,
};

use std::rc::Rc;

pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(32),
        }
    }

    #[inline]
    fn push(
        &mut self,
        value: Value,
    ) {
        self.stack.push(value);
    }

    #[inline]
    fn pop(
        &mut self,
    ) -> Result<Value> {
        self.stack.pop().ok_or_else(|| {
            Error::new(
                ErrorKind::Runtime,
                "VM stack underflow",
                None,
            )
        })
    }

    #[inline]
    fn pop_bool(
        &mut self,
    ) -> Result<bool> {
        match self.pop()? {
            Value::Bool(value) =>
                Ok(value),

            other =>
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "expected Bool, got {}",
                            other.type_name()
                        ),
                        None,
                    )
                ),
        }
    }

    pub fn run(
        &mut self,
        chunk: Rc<Chunk>,
    ) -> Result<Value> {
        self.stack.clear();
        self.frames.clear();

        self.frames.push(
            CallFrame {
                chunk,
                ip: 0,
                locals: Vec::new(),
            }
        );

        self.execute()
    }

    fn execute(
        &mut self,
    ) -> Result<Value> {
        loop {
            let (
                opcode,
                operand,
            ) = {
                let frame =
                    self.frames
                        .last()
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                "VM has no call frame",
                                None,
                            )
                        })?;

                let instruction =
                    frame
                        .chunk
                        .code
                        .get(frame.ip)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                "instruction pointer out of bounds",
                                None,
                            )
                        })?
                        .clone();

                (
                    instruction.opcode,
                    instruction.operand,
                )
            };

            self.current_frame_mut().ip += 1;

            match opcode {
                OpCode::Constant => {
                    let value = {
                        let frame =
                            self.current_frame();

                        frame
                            .chunk
                            .constants
                            .get(
                                operand as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "constant index out of bounds",
                                    None,
                                )
                            })?
                    };

                    self.push(value);
                }

                OpCode::Unit => {
                    self.push(
                        Value::Unit
                    );
                }

                OpCode::Pop => {
                    self.pop()?;
                }

                OpCode::Dup => {
                    let value =
                        self.stack
                            .last()
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "VM stack underflow",
                                    None,
                                )
                            })?;

                    self.push(value);
                }

                OpCode::Add => {
                    self.binary_numeric(
                        OpCode::Add
                    )?;
                }

                OpCode::Sub => {
                    self.binary_numeric(
                        OpCode::Sub
                    )?;
                }

                OpCode::Mul => {
                    self.binary_numeric(
                        OpCode::Mul
                    )?;
                }

                OpCode::Div => {
                    self.binary_numeric(
                        OpCode::Div
                    )?;
                }

                OpCode::Mod => {
                    self.binary_numeric(
                        OpCode::Mod
                    )?;
                }

                OpCode::Neg => {
                    let value =
                        self.pop()?;

                    self.push(
                        value
                            .negate()
                            .map_err(|message| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                )
                            })?
                    );
                }

                OpCode::Not => {
                    let value =
                        self.pop()?;

                    match value {
                        Value::Bool(value) => {
                            self.push(
                                Value::Bool(
                                    !value
                                )
                            );
                        }

                        other => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    format!(
                                        "expected Bool, got {}",
                                        other.type_name()
                                    ),
                                    None,
                                )
                            );
                        }
                    }
                }

                OpCode::Jump => {
                    self.current_frame_mut().ip =
                        operand as usize;
                }

                OpCode::JumpIfFalse => {
                    let condition =
                        self.pop_bool()?;

                    if !condition {
                        self.current_frame_mut().ip =
                            operand as usize;
                    }
                }

                OpCode::LoadLocal => {
                    let index =
                        operand as usize;

                    let value =
                        self.current_frame()
                            .locals
                            .get(index)
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    format!(
                                        "local slot out of bounds: {}",
                                        index
                                    ),
                                    None,
                                )
                            })?;

                    self.push(value);
                }

                OpCode::StoreLocal => {
                    let index =
                        operand as usize;

                    let value =
                        self.pop()?;

                    let frame =
                        self.current_frame_mut();

                    if index >= frame.locals.len() {
                        frame.locals.resize(
                            index + 1,
                            Value::Unit,
                        );
                    }

                    frame.locals[index] =
                        value;
                }

                OpCode::Call => {
                    let argc =
                        operand as usize;

                    let function_index =
                        self.stack
                            .len()
                            .checked_sub(
                                argc + 1
                            )
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid call stack",
                                    None,
                                )
                            })?;

                    let function =
                        self.stack[
                            function_index
                        ].clone();

                    let Value::VmFunction(
                        function
                    ) = function
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "value is not a VM function",
                                None,
                            )
                        );
                    };

                    if function.arity
                        as usize
                        != argc
                    {
                        return Err(
                            Error::new(
                                ErrorKind::Arity,
                                format!(
                                    "function expects {} arguments, got {}",
                                    function.arity,
                                    argc
                                ),
                                None,
                            )
                        );
                    }

                    let args_start =
                        function_index + 1;

                    let args =
                        self.stack[
                            args_start..
                        ].to_vec();

                    self.stack.truncate(
                        function_index
                    );

                    self.frames.push(
                        CallFrame {
                            chunk:
                                function.chunk.clone(),
                            ip: 0,
                            locals: args,
                        }
                    );
                }

                OpCode::Return => {
                    let result =
                        self.pop()?;

                    self.frames.pop()
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                "VM frame underflow",
                                None,
                            )
                        })?;

                    if self.frames.is_empty() {
                        return Ok(result);
                    }

                    self.push(result);
                }

                OpCode::Halt => {
                    return self.pop();
                }

                _ => {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            format!(
                                "opcode {:?} is not implemented",
                                opcode
                            ),
                            None,
                        )
                    );
                }
            }
        }
    }

    fn binary_numeric(
        &mut self,
        op: OpCode,
    ) -> Result<()> {
        let right =
            self.pop()?;

        let left =
            self.pop()?;

        let result =
            match (left, right) {
                (
                    Value::Int(a),
                    Value::Int(b),
                ) => {
                    match op {
                        OpCode::Add =>
                            a.checked_add(b)
                                .map(Value::Int)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "integer overflow",
                                        None,
                                    )
                                })?,

                        OpCode::Sub =>
                            a.checked_sub(b)
                                .map(Value::Int)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "integer overflow",
                                        None,
                                    )
                                })?,

                        OpCode::Mul =>
                            a.checked_mul(b)
                                .map(Value::Int)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "integer overflow",
                                        None,
                                    )
                                })?,

                        OpCode::Mod => {
                            if b == 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "division by zero",
                                        None,
                                    )
                                );
                            }

                            Value::Int(
                                a % b
                            )
                        }

                        OpCode::Div => {
                            if b == 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "division by zero",
                                        None,
                                    )
                                );
                            }

                            Value::Float(
                                a as f64 / b as f64
                            )
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid numeric opcode",
                                    None,
                                )
                            );
                        }
                    }
                }

                (
                    Value::Float(a),
                    Value::Float(b),
                ) => {
                    match op {
                        OpCode::Add =>
                            Value::Float(a + b),

                        OpCode::Sub =>
                            Value::Float(a - b),

                        OpCode::Mul =>
                            Value::Float(a * b),

                        OpCode::Div =>
                            Value::Float(a / b),

                        OpCode::Mod =>
                            Value::Float(a % b),

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid numeric opcode",
                                    None,
                                )
                            );
                        }
                    }
                }

                (
                    Value::Int(a),
                    Value::Float(b),
                ) => {
                    let a =
                        a as f64;

                    match op {
                        OpCode::Add =>
                            Value::Float(a + b),

                        OpCode::Sub =>
                            Value::Float(a - b),

                        OpCode::Mul =>
                            Value::Float(a * b),

                        OpCode::Div =>
                            Value::Float(a / b),

                        OpCode::Mod =>
                            Value::Float(a % b),

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid numeric opcode",
                                    None,
                                )
                            );
                        }
                    }
                }

                (
                    Value::Float(a),
                    Value::Int(b),
                ) => {
                    let b =
                        b as f64;

                    match op {
                        OpCode::Add =>
                            Value::Float(a + b),

                        OpCode::Sub =>
                            Value::Float(a - b),

                        OpCode::Mul =>
                            Value::Float(a * b),

                        OpCode::Div =>
                            Value::Float(a / b),

                        OpCode::Mod =>
                            Value::Float(a % b),

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid numeric opcode",
                                    None,
                                )
                            );
                        }
                    }
                }

                (a, b) => {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "numeric operation not defined between {} and {}",
                                a.type_name(),
                                b.type_name()
                            ),
                            None,
                        )
                    );
                }
            };

        self.push(result);

        Ok(())
    }

    fn binary_compare<F>(
        &mut self,
        op: F,
    ) -> Result<()>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let right =
            self.pop()?;

        let left =
            self.pop()?;

        let left =
            match left {
                Value::Int(value) =>
                    value as f64,

                Value::Float(value) =>
                    value,

                other =>
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "expected numeric value, got {}",
                                other.type_name()
                            ),
                            None,
                        )
                    ),
            };

        let right =
            match right {
                Value::Int(value) =>
                    value as f64,

                Value::Float(value) =>
                    value,

                other =>
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "expected numeric value, got {}",
                                other.type_name()
                            ),
                            None,
                        )
                    ),
            };

        self.push(
            Value::Bool(
                op(left, right)
            )
        );

        Ok(())
    }

    #[inline]
    fn current_frame(
        &self,
    ) -> &CallFrame {
        self.frames
            .last()
            .expect(
                "VM has no current frame"
            )
    }

    #[inline]
    fn current_frame_mut(
        &mut self,
    ) -> &mut CallFrame {
        self.frames
            .last_mut()
            .expect(
                "VM has no current frame"
            )
    }

}