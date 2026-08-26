use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    }, 
    runtime::{
        List,
        Value,
        IteratorObj,
        IteratorRef,
        IterResult,
        operator, 
    },
    syntax::BinOp, 
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
    decode_method_call,
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
        self.execute_until_depth(0)
    }

    fn execute_until_depth(
        &mut self,
        target_depth: usize,
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

                OpCode::InvokeMethod => {
                    let (
                        method_index,
                        argc,
                    ) =
                        decode_method_call(
                            operand
                        );

                    let method =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .constants
                            .get(
                                method_index as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "method name constant out of bounds",
                                    None,
                                )
                            })?;

                    let Value::Str(
                        method
                    ) = method
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "method name constant must be Str",
                                None,
                            )
                        );
                    };

                    let argc =
                        argc as usize;

                    let receiver_index =
                        self.stack
                            .len()
                            .checked_sub(
                                argc + 1
                            )
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "invalid method call stack",
                                    None,
                                )
                            })?;

                    let receiver =
                        self.stack[
                            receiver_index
                        ]
                        .clone();

                    let args =
                        self.stack[
                            receiver_index + 1..
                        ]
                        .to_vec();

                    self.stack.truncate(
                        receiver_index
                    );

                    let result =
                        self.invoke_method(
                            receiver,
                            method.as_str(),
                            args,
                        )?;

                    self.push(result);
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

                OpCode::NewList => {
                    let list =
                        Rc::new(
                            List::with_capacity(
                                operand as usize
                            )
                        );

                    self.push(
                        Value::List(
                            list
                        )
                    );
                }

                OpCode::ListAppend => {
                    let value =
                        self.pop()?;

                    let list =
                        self.stack
                            .last()
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "list append stack underflow",
                                    None,
                                )
                            })?;

                    let Value::List(
                        list
                    ) = list
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "LIST_APPEND expects a List",
                                None,
                            )
                        );
                    };

                    list.push(value);
                }

                OpCode::ListExtendRange => {
                    let end = self.pop()?;
                    let start = self.pop()?;

                    let list =
                        match self.stack.last().cloned() {
                            Some(Value::List(list)) =>
                                list,

                            Some(other) =>
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        format!(
                                            "LIST_EXTEND_RANGE expects List, got {}",
                                            other.type_name()
                                        ),
                                        None,
                                    )
                                ),

                            None =>
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "list range stack underflow",
                                        None,
                                    )
                                ),
                        };

                    let start =
                        match start {
                            Value::Int(value) => value,
                            other => {
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        format!(
                                            "range start must be Int, got {}",
                                            other.type_name()
                                        ),
                                        None,
                                    )
                                );
                            }
                        };

                    let end =
                        match end {
                            Value::Int(value) => value,
                            other => {
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        format!(
                                            "range end must be Int, got {}",
                                            other.type_name()
                                        ),
                                        None,
                                    )
                                );
                            }
                        };

                    let inclusive =
                        operand != 0;

                    let end =
                        if inclusive {
                            end.checked_add(1)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Overflow,
                                        "range upper bound overflow",
                                        None,
                                    )
                                })?
                        } else {
                            end
                        };

                    if start < end {
                        let count =
                            end.checked_sub(start)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Overflow,
                                        "range length overflow",
                                        None,
                                    )
                                })?;

                        let mut list =
                            list.as_vec_mut();

                        list.reserve(
                            count as usize
                        );

                        for value in start..end {
                            list.push(
                                Value::Int(value)
                            );
                        }
                    }
                }

                OpCode::IndexGet => {
                    let index =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    match (object, index) {
                        (
                            Value::List(list),
                            Value::Int(index),
                        ) => {
                            if index < 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Index,
                                        "list index must be non-negative",
                                        None,
                                    )
                                );
                            }

                            let index =
                                index as usize;

                            let value =
                                list.get(index)
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            format!(
                                                "list index out of bounds: {}",
                                                index
                                            ),
                                            None,
                                        )
                                    })?;

                            self.push(value);
                        }

                        (
                            _,
                            _
                        ) => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "unsupported indexing operation",
                                    None,
                                )
                            );
                        }
                    }
                }

                OpCode::IndexSet => {
                    let index =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    let value =
                        self.pop()?;

                    match (object, index) {
                        (
                            Value::List(list),
                            Value::Int(index),
                        ) => {
                            if index < 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Index,
                                        "list index must be non-negative",
                                        None,
                                    )
                                );
                            }

                            list.set(
                                index as usize,
                                value.clone(),
                            )
                            .map_err(|message| {
                                Error::new(
                                    ErrorKind::Index,
                                    message,
                                    None,
                                )
                            })?;

                            self.push(value);
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "unsupported indexed assignment",
                                    None,
                                )
                            );
                        }
                    }
                }

                OpCode::IteratorFrom => {
                    let value =
                        self.pop()?;

                    let iterator =
                        IteratorObj::from_value(
                            value
                        )
                        .map_err(|message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        })?;

                    self.push(
                        Value::Iterator(
                            iterator
                        )
                    );
                }

                OpCode::IteratorNext => {
                    let value =
                        self.pop()?;

                    let Value::Iterator(
                        iterator
                    ) = value
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "IteratorNext expects Iterator",
                                None,
                            )
                        );
                    };

                    match self.iterator_next(
                        iterator
                    )? {
                        IterResult::Item(value) => {
                            self.push(value);

                            self.push(
                                Value::Bool(true)
                            );
                        }

                        IterResult::End => {
                            self.push(
                                Value::Unit
                            );

                            self.push(
                                Value::Bool(false)
                            );
                        }
                    }
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

                    if self.frames.len() ==
                        target_depth
                    {
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
            operator::apply_binop(
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

    fn call_closure_sync(
        &mut self,
        closure: ClosureRef,
        args: Vec<Value>,
    ) -> Result<Value> {
        let caller_depth =
            self.frames.len();

        let local_count =
            closure
                .function
                .chunk
                .local_count;

        let mut locals =
            args;

        locals.resize(
            local_count,
            Value::Unit,
        );

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells:
                    vec![
                        None;
                        local_count
                    ],
            }
        );

        self.execute_until_depth(
            caller_depth
        )
    }

    fn iterator_next(
        &mut self,
        iterator: IteratorRef,
    ) -> Result<IterResult> {
        let kind =
            iterator.borrow().clone();

        match kind {
            IteratorObj::List { .. }
            | IteratorObj::Str { .. }
            | IteratorObj::Vector { .. }
            | IteratorObj::Range { .. }
            | IteratorObj::Enumerate { .. }
            | IteratorObj::Zip { .. }
            | IteratorObj::Take { .. }
            | IteratorObj::Skip { .. }
            => {
                IteratorObj::next(
                    &iterator
                )
                .map_err(|message| {
                    Error::new(
                        ErrorKind::Runtime,
                        message,
                        None,
                    )
                })
            }

            IteratorObj::Map {
                source,
                function,
            } => {
                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        self.call_closure_sync(
                            function,
                            vec![value],
                        )
                        .map(
                            IterResult::Item
                        )
                    }
                }
            }

            IteratorObj::Filter {
                source,
                predicate,
            } => {
                loop {
                    match self.iterator_next(
                        source.clone()
                    )? {
                        IterResult::End =>
                            return Ok(
                                IterResult::End
                            ),

                        IterResult::Item(value) => {
                            let result =
                                self.call_closure_sync(
                                    predicate.clone(),
                                    vec![value.clone()],
                                )?;

                            let Value::Bool(
                                keep
                            ) = result
                            else {
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        "filter predicate must return Bool",
                                        None,
                                    )
                                );
                            };

                            if keep {
                                return Ok(
                                    IterResult::Item(
                                        value
                                    )
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Handles any type of method dispatches
    /// List, Struct, Object, ... etc
    fn invoke_method(
        &mut self,
        receiver: Value,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match receiver {
            Value::List(list) => {
                self.invoke_list_method(
                    list,
                    name,
                    args,
                )
            }

            Value::Iterator(iterator) => {
                self.invoke_iterator_method(
                    iterator,
                    name,
                    args,
                )
            }

            Value::Range(
                start,
                end,
                inclusive,
            ) => {
                self.invoke_range_method(
                    start,
                    end,
                    inclusive,
                    name,
                    args,
                )
            }

            Value::Str(string) => {
                self.invoke_string_method(
                    string,
                    name,
                    args,
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "method '{}' is not supported for this value",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn invoke_string_method(
        &mut self,
        string: crate::runtime::StrRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        todo!()
    }

    fn invoke_list_method(
        &mut self,
        list: crate::runtime::ListRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "len" => {
                if !args.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "len() expects no arguments",
                            None,
                        )
                    );
                }

                Ok(
                    Value::Int(
                        list.len() as i64
                    )
                )
            }

            "push" => {
                if args.len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "push() expects one argument",
                            None,
                        )
                    );
                }

                list.push(
                    args[0].clone()
                );

                Ok(
                    Value::Unit
                )
            }

            "iter" => {
                if !args.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "iter() expects no arguments",
                            None,
                        )
                    );
                }

                let iterator =
                    IteratorObj::from_value(
                        Value::List(list)
                    )
                    .map_err(|message| {
                        Error::new(
                            ErrorKind::Type,
                            message,
                            None,
                        )
                    })?;

                Ok(
                    Value::Iterator(
                        iterator
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "List has no method '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn invoke_iterator_method(
        &mut self,
        iterator: IteratorRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "next" => {
                if !args.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "next() expects no arguments",
                            None,
                        )
                    );
                }

                match self.iterator_next(
                    iterator
                )? {
                    IterResult::Item(value) => {
                        Ok(
                            Value::Tuple(
                                Rc::new(vec![
                                    value,
                                    Value::Bool(true),
                                ])
                            )
                        )
                    }

                    IterResult::End => {
                        Ok(
                            Value::Tuple(
                                Rc::new(vec![
                                    Value::Unit,
                                    Value::Bool(false),
                                ])
                            )
                        )
                    }
                }
            }

            "map" => {
                if args.len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "map() expects one argument",
                            None,
                        )
                    );
                }

                let Value::Closure(
                    closure
                ) = args[0].clone()
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            "map() expects a function",
                            None,
                        )
                    );
                };

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Map {
                                    source: iterator,
                                    function: closure,
                                }
                            )
                        )
                    )
                )
            }

            "filter" => {
                if args.len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "filter() expects one argument",
                            None,
                        )
                    );
                }

                let Value::Closure(
                    closure
                ) = args[0].clone()
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            "filter() expects a function",
                            None,
                        )
                    );
                };

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Filter {
                                    source: iterator,
                                    predicate: closure,
                                }
                            )
                        )
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "Iterator has no method '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn invoke_range_method(
        &mut self,
        start: i64,
        end: i64,
        inclusive: bool,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "iter" => {
                if !args.is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Arity,
                            "iter() expects no arguments",
                            None,
                        )
                    );
                }

                let iterator =
                    IteratorObj::from_value(
                        Value::Range(
                            start,
                            end,
                            inclusive,
                        )
                    )
                    .map_err(|message| {
                        Error::new(
                            ErrorKind::Type,
                            message,
                            None,
                        )
                    })?;

                Ok(
                    Value::Iterator(
                        iterator
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "Range has no method '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
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