use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    }, interpreter, runtime::{Value}, syntax::BinOp,
};

use super::{
    Chunk,
    OpCode,
    CallFrame,
    FunctionProto,
    FunctionRef,
    Closure,
    ClosureRef,
    Instruction,
    UpvalueSpec,
    CellRef,
};

use std::{
    rc::Rc,
    cell::RefCell,
};

pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    repl_locals: Vec<Value>,
    repl_cells: Vec<Option<CellRef>>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(32),
            repl_locals: Vec::with_capacity(64),
            repl_cells: Vec::with_capacity(64),
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

        let function =
            Rc::new(
                FunctionProto {
                    arity: 0,
                    chunk: chunk.clone(),
                    upvalue_specs: Vec::new(),
                }
            );

        let closure =
            Rc::new(
                Closure {
                    function,
                    upvalues: Vec::new(),
                }
            );

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals: Vec::new(),
                cells: Vec::new(),
            }
        );

        self.execute()
    }

    pub fn run_repl(
        &mut self,
        chunk: Rc<Chunk>,
    ) -> Result<Value> {
        self.stack.clear();
        self.frames.clear();

        let function =
            Rc::new(
                FunctionProto {
                    arity: 0,
                    chunk: chunk.clone(),
                    upvalue_specs: Vec::new(),
                }
            );

        let closure =
            Rc::new(
                Closure {
                    function,
                    upvalues: Vec::new(),
                }
            );

        let locals =
            std::mem::take(
                &mut self.repl_locals
            );

        let cells =
            std::mem::take(
                &mut self.repl_cells
            );

        let local_count =
            chunk.local_count;

        let mut locals =
            locals;

        let mut cells =
            cells;

        if locals.len() <
            local_count
        {
            locals.resize(
                local_count,
                Value::Unit,
            );
        }

        if cells.len() <
            local_count
        {
            cells.resize(
                local_count,
                None,
            );
        }

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells,
            }
        );

        let result =
            self.execute();

        match self.frames.pop() {
            Some(frame) => {
                self.repl_locals =
                    frame.locals;

                self.repl_cells =
                    frame.cells;
            }

            None => {
                self.repl_locals.clear();
                self.repl_cells.clear();
            }
        }

        result
    }

    fn execute(
        &mut self,
    ) -> Result<Value> {
        loop {
            let instruction =
            self.fetch_instruction()?;

            let opcode =
                instruction.opcode;

            let operand =
                instruction.operand;

            match opcode {
                OpCode::Constant => {
                    let value =
                        self.current_frame()
                            .closure
                            .function
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
                            })?;

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

                OpCode::Add
                    | OpCode::Sub
                    | OpCode::Mul
                    | OpCode::Div
                    | OpCode::Mod
                    | OpCode::Pow
                    | OpCode::MatMul
                    | OpCode::Eq
                    | OpCode::Neq
                    | OpCode::Lt
                    | OpCode::Leq
                    | OpCode::Gt
                    | OpCode::Geq => {
                        let binop =
                            Self::opcode_to_binop(
                                opcode
                            )
                            .expect(
                                "numeric opcode must map to BinOp"
                            );

                        self.binary_op(
                            binop
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
                    let slot =
                        operand as usize;

                    let frame =
                        self.current_frame();

                    let value =
                        if let Some(
                            Some(cell)
                        ) = frame.cells.get(slot)
                        {
                            cell.borrow().clone()
                        } else {
                            frame
                                .locals
                                .get(slot)
                                .cloned()
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Runtime,
                                        format!(
                                            "local slot out of bounds: {}",
                                            slot
                                        ),
                                        None,
                                    )
                                })?
                        };

                    self.push(value);
                }

                OpCode::StoreLocal => {
                    let slot =
                        operand as usize;

                    let value =
                        self.pop()?;

                    let frame =
                        self.current_frame_mut();

                    if slot >= frame.locals.len() {
                        frame.locals.resize(
                            slot + 1,
                            Value::Unit,
                        );
                    }

                    if frame.cells.len() <= slot {
                        frame.cells.resize(
                            slot + 1,
                            None,
                        );
                    }

                    if let Some(cell) =
                        frame.cells[slot].clone()
                    {
                        *cell.borrow_mut() =
                            value.clone();
                    }

                    frame.locals[slot] =
                        value;
                }

                OpCode::ResetLocal => {
                    let slot =
                        operand as usize;

                    let frame =
                        self.current_frame_mut();

                    if slot >= frame.locals.len() {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                format!(
                                    "local slot out of bounds: {}",
                                    slot
                                ),
                                None,
                            )
                        );
                    }

                    // A captured iteration binding keeps its cell alive
                    // outside the iteration. The next iteration receives
                    // a fresh cell.
                    frame.cells[slot] =
                        None;

                    frame.locals[slot] =
                        Value::Unit;
                }

                OpCode::LoadUpvalue => {
                    let index =
                        operand as usize;

                    let cell =
                        self.current_frame()
                            .closure
                            .upvalues
                            .get(index)
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "upvalue slot out of bounds",
                                    None,
                                )
                            })?;

                    self.push(
                        cell.borrow().clone()
                    );
                }

                OpCode::StoreUpvalue => {
                    let index =
                        operand as usize;

                    let value =
                        self.pop()?;

                    let cell =
                        self.current_frame()
                            .closure
                            .upvalues
                            .get(index)
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "upvalue slot out of bounds",
                                    None,
                                )
                            })?;

                    *cell.borrow_mut() =
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

                    let value =
                        self.stack[
                            function_index
                        ].clone();

                    let Value::Closure(
                        closure
                    ) = value
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "value is not callable",
                                None,
                            )
                        );
                    };

                    if closure
                        .function
                        .arity as usize
                        != argc
                    {
                        return Err(
                            Error::new(
                                ErrorKind::Arity,
                                format!(
                                    "function expects {} arguments, got {}",
                                    closure.function.arity,
                                    argc
                                ),
                                None,
                            )
                        );
                    }

                    let args =
                        self.stack[
                            function_index + 1..
                        ]
                        .to_vec();

                    self.stack.truncate(
                        function_index
                    );

                    let local_count =
                        closure
                            .function
                            .chunk
                            .local_count;

                    self.frames.push(
                        CallFrame {
                            closure,
                            ip: 0,
                            locals: {
                                let mut locals =
                                    args;

                                locals.resize(
                                    local_count,
                                    Value::Unit,
                                );

                                locals
                            },
                            cells:
                                vec![
                                    None;
                                    local_count
                                ],
                        }
                    );
                }

                OpCode::Closure => {
                    let value =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .constants
                            .get(
                                operand as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "closure prototype constant out of bounds",
                                    None,
                                )
                            })?;

                    let Value::FunctionProto(
                        function
                    ) = value
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "closure opcode requires a function prototype",
                                None,
                            )
                        );
                    };

                    let closure =
                        self.create_closure(
                            function
                        )?;

                    self.push(
                        Value::Closure(
                            closure
                        )
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

    #[inline]
    fn fetch_instruction(
        &mut self,
    ) -> Result<Instruction> {
        let frame =
            self.current_frame_mut();

        let instruction =
            frame
                .closure
                .function
                .chunk
                .code
                .get(frame.ip)
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "instruction pointer out of bounds",
                        None,
                    )
                })?;

        frame.ip += 1;

        Ok(instruction)
    }

    #[inline]
    fn opcode_to_binop(
        opcode: OpCode,
    ) -> Option<BinOp> {
        match opcode {
            OpCode::Add =>
                Some(BinOp::Add),

            OpCode::Sub =>
                Some(BinOp::Sub),

            OpCode::Mul =>
                Some(BinOp::Mul),

            OpCode::Div =>
                Some(BinOp::Div),

            OpCode::Mod =>
                Some(BinOp::Mod),

            OpCode::Pow =>
                Some(BinOp::Pow),

            OpCode::Eq =>
                Some(BinOp::Eq),

            OpCode::Neq =>
                Some(BinOp::Neq),

            OpCode::Lt =>
                Some(BinOp::Lt),

            OpCode::Leq =>
                Some(BinOp::Leq),

            OpCode::Gt =>
                Some(BinOp::Gt),

            OpCode::Geq =>
                Some(BinOp::Geq),

            _ =>
                None,
        }
    }

    fn binary_op(
        &mut self,
        op: BinOp,
    ) -> Result<()> {
        let right = self.pop()?;
        let left = self.pop()?;

        let result =
            interpreter::apply_binop(
                op,
                left,
                right,
            )
            .map_err(|message| {
                Error::new(
                    ErrorKind::Runtime,
                    message,
                    None,
                )
            })?;

        self.push(result);

        Ok(())
    }

    fn capture_local(
        &mut self,
        slot: usize,
    ) -> Result<CellRef> {
        let frame =
            self.current_frame_mut();

        if slot >= frame.locals.len() {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    format!(
                        "capture local slot out of bounds: {}",
                        slot
                    ),
                    None,
                )
            );
        }

        if frame.cells.len() <= slot {
            frame.cells.resize(
                slot + 1,
                None,
            );
        }

        if let Some(cell) =
            frame.cells[slot].clone()
        {
            return Ok(cell);
        }

        let cell =
            Rc::new(
                RefCell::new(
                    frame.locals[slot]
                        .clone()
                )
            );

        frame.cells[slot] =
            Some(
                cell.clone()
            );

        Ok(cell)
    }

    fn create_closure(
        &mut self,
        function: FunctionRef,
    ) -> Result<ClosureRef> {
        let mut upvalues =
            Vec::with_capacity(
                function.upvalue_specs.len()
            );

        for spec in
            &function.upvalue_specs
        {
            let cell =
                match *spec {
                    UpvalueSpec::Local(
                        slot
                    ) => {
                        self.capture_local(
                            slot as usize
                        )?
                    }

                    UpvalueSpec::Parent(
                        index
                    ) => {
                        self.current_frame()
                            .closure
                            .upvalues
                            .get(
                                index as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "parent upvalue out of bounds",
                                    None,
                                )
                            })?
                    }
                };

            upvalues.push(
                cell
            );
        }

        Ok(
            Rc::new(
                Closure {
                    function,
                    upvalues,
                }
            )
        )
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