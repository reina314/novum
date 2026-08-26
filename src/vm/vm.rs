use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    }, 
    runtime::{
        List,
        ListRef,
        Value,
        EnumValue,
        EnumConstructor,
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

                    let callable =
                        self.stack[
                            function_index
                        ].clone();

                    match callable {
                        Value::Closure(
                            closure
                        ) => {
                            self.call_closure_frame(
                                function_index,
                                closure,
                                argc,
                            )?;
                        }

                        Value::EnumConstructor(
                            constructor
                        ) => {
                            self.call_enum_constructor(
                                function_index,
                                constructor,
                                argc,
                            )?;
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "value is not callable",
                                    None,
                                )
                            );
                        }
                    }
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

                OpCode::NewTuple => {
                    let count =
                        operand as usize;

                    if self.stack.len() < count {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM stack underflow while creating tuple",
                                None,
                            )
                        );
                    }

                    let start =
                        self.stack.len() - count;

                    let values =
                        self.stack
                            .drain(start..)
                            .collect::<Vec<_>>();

                    self.push(
                        Value::Tuple(
                            Rc::new(values)
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

                OpCode::NewRange => {
                    let end =
                        self.pop()?;

                    let start =
                        self.pop()?;

                    let start =
                        match start {
                            Value::Int(value) =>
                                value,

                            other =>
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        format!(
                                            "range start must be Int, got {}",
                                            other.type_name()
                                        ),
                                        None,
                                    )
                                ),
                        };

                    let end =
                        match end {
                            Value::Int(value) =>
                                value,

                            other =>
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        format!(
                                            "range end must be Int, got {}",
                                            other.type_name()
                                        ),
                                        None,
                                    )
                                ),
                        };

                    self.push(
                        Value::Range(
                            start,
                            end,
                            operand != 0,
                        )
                    );
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

                            let value =
                                list.get(
                                    index as usize
                                )
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Index,
                                        "list index out of bounds",
                                        None,
                                    )
                                })?;

                            self.push(value);
                        }

                        (
                            Value::Tuple(tuple),
                            Value::Int(index),
                        ) => {
                            if index < 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Index,
                                        "tuple index must be non-negative",
                                        None,
                                    )
                                );
                            }

                            let value =
                                tuple
                                    .get(index as usize)
                                    .cloned()
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            "tuple index out of bounds",
                                            None,
                                        )
                                    })?;

                            self.push(value);
                        }

                        _ => {
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

                OpCode::FieldGet => {
                    let field =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    let Value::Str(
                        field
                    ) = field
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "field name must be Str",
                                None,
                            )
                        );
                    };

                    match object {
                        Value::Enum(
                            enum_def
                        ) => {
                            if enum_def
                                .variant(
                                    field.as_str()
                                )
                                .is_none()
                            {
                                return Err(
                                    Error::new(
                                        ErrorKind::Name,
                                        format!(
                                            "enum '{}' has no variant '{}'",
                                            enum_def.name(),
                                            field
                                        ),
                                        None,
                                    )
                                );
                            }

                            self.push(
                                Value::EnumConstructor(
                                    EnumConstructor::new(
                                        enum_def,
                                        field.as_str(),
                                    )
                                )
                            );
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "unsupported field access",
                                    None,
                                )
                            );
                        }
                    }
                }

                OpCode::EnumFieldGet => {
                    let index =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    let Value::EnumValue(
                        value
                    ) = object
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "EnumFieldGet expects EnumValue",
                                None,
                            )
                        );
                    };

                    let Value::Int(
                        index
                    ) = index
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "enum field index must be Int",
                                None,
                            )
                        );
                    };

                    if index < 0 {
                        return Err(
                            Error::new(
                                ErrorKind::Index,
                                "enum field index must be non-negative",
                                None,
                            )
                        );
                    }

                    let field =
                        value.field(
                            index as usize
                        )
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Index,
                                "enum field index out of bounds",
                                None,
                            )
                        })?;

                    self.push(field);
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

                OpCode::MatchTuple => {
                    let value =
                        self.pop()?;

                    let matched =
                        matches!(
                            &value,
                            Value::Tuple(tuple)
                                if tuple.len()
                                    == operand as usize
                        );

                    self.push(value);
                    self.push(
                        Value::Bool(matched)
                    );
                }

                OpCode::MatchEnum => {
                    let variant =
                        self.pop()?;

                    let enum_name =
                        self.pop()?;

                    let value =
                        self.pop()?;

                    let Value::Str(
                        enum_name
                    ) = enum_name
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "MatchEnum expects enum name",
                                None,
                            )
                        );
                    };

                    let Value::Str(
                        variant
                    ) = variant
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "MatchEnum expects variant name",
                                None,
                            )
                        );
                    };

                    let matched =
                        match &value {
                            Value::EnumValue(
                                value
                            ) => {
                                value.enum_name()
                                    == enum_name.as_str()
                                    && value.variant()
                                        == variant.as_str()
                                    && value.fields().len()
                                        == operand as usize
                            }

                            _ => false,
                        };

                    self.push(value);

                    self.push(
                        Value::Bool(
                            matched
                        )
                    );
                }

                OpCode::MatchList => {
                    let value =
                        self.pop()?;

                    let matched =
                        matches!(
                            &value,
                            Value::List(list)
                                if list.len()
                                    == operand as usize
                        );

                    self.push(value);
                    self.push(
                        Value::Bool(matched)
                    );
                }

                OpCode::PatternFail => {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "match expression has no matching arm",
                            None,
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

    fn call_closure_frame(
        &mut self,
        function_index: usize,
        closure: ClosureRef,
        argc: usize,
    ) -> Result<()> {
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

        Ok(())
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

    fn call_enum_constructor(
        &mut self,
        function_index: usize,
        constructor: EnumConstructor,
        argc: usize,
    ) -> Result<()> {
        let expected =
            constructor.arity();

        if argc != expected {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{} expects {} arguments, got {}",
                        constructor,
                        expected,
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

        let value =
            EnumValue::new(
                constructor
                    .enum_def()
                    .name(),
                constructor
                    .variant(),
                args,
            );

        self.push(
            Value::EnumValue(
                Rc::new(value)
            )
        );

        Ok(())
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

    // ============================
    //   Iterator
    // ============================
    fn iterator_next(
        &mut self,
        iterator: IteratorRef,
    ) -> Result<IterResult> {
        let state =
            iterator.borrow().clone();

        match state {
            // Base iterators do not require VM execution.
            IteratorObj::List { .. }
            | IteratorObj::Str { .. }
            | IteratorObj::Vector { .. }
            | IteratorObj::Range { .. } => {
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

            // Lazy transformation.
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
                        let result =
                            self.call_closure_sync(
                                function,
                                vec![value],
                            )?;

                        Ok(
                            IterResult::Item(
                                result
                            )
                        )
                    }
                }
            }

            // Lazy filtering.
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

                            match result {
                                Value::Bool(true) => {
                                    return Ok(
                                        IterResult::Item(
                                            value
                                        )
                                    );
                                }

                                Value::Bool(false) => {
                                    continue;
                                }

                                other => {
                                    return Err(
                                        Error::new(
                                            ErrorKind::Type,
                                            format!(
                                                "filter predicate must return Bool, got {}",
                                                other.type_name()
                                            ),
                                            None,
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Add an index to each item.
            IteratorObj::Enumerate {
                source,
                index: _,
            } => {
                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        if let IteratorObj::Enumerate {
                            index,
                            ..
                        } = &mut *iterator.borrow_mut()
                        {
                            let current =
                                *index;

                            *index += 1;

                            Ok(
                                IterResult::Item(
                                    Value::Tuple(
                                        Rc::new(vec![
                                            Value::Int(
                                                current as i64
                                            ),
                                            value,
                                        ])
                                    )
                                )
                            )
                        } else {
                            unreachable!();
                        }
                    }
                }
            }

            // Zip terminates when either source terminates.
            IteratorObj::Zip {
                left,
                right,
            } => {
                let left_value =
                    self.iterator_next(
                        left
                    )?;

                let right_value =
                    self.iterator_next(
                        right
                    )?;

                match (
                    left_value,
                    right_value,
                ) {
                    (
                        IterResult::Item(left),
                        IterResult::Item(right),
                    ) => {
                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(vec![
                                        left,
                                        right,
                                    ])
                                )
                            )
                        )
                    }

                    _ =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            // Take at most `remaining` items.
            IteratorObj::Take {
                source,
                remaining,
            } => {
                if remaining == 0 {
                    return Ok(
                        IterResult::End
                    );
                }

                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        if let IteratorObj::Take {
                            remaining,
                            ..
                        } = &mut *iterator.borrow_mut()
                        {
                            *remaining -= 1;
                        }

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }
                }
            }

            // Skip the first `remaining` items, then pass through.
            IteratorObj::Skip {
                source,
                remaining,
            } => {
                let mut remaining =
                    remaining;

                while remaining > 0 {
                    match self.iterator_next(
                        source.clone()
                    )? {
                        IterResult::End =>
                            return Ok(
                                IterResult::End
                            ),

                        IterResult::Item(_) => {
                            remaining -= 1;
                        }
                    }
                }

                if let IteratorObj::Skip {
                    remaining: state_remaining,
                    ..
                } = &mut *iterator.borrow_mut()
                {
                    *state_remaining = 0;
                }

                self.iterator_next(
                    source
                )
            }
        }
    }

    fn collect_iterator(
        &mut self,
        iterator: IteratorRef,
    ) -> Result<Value> {
        let list =
            Rc::new(
                List::with_capacity(0)
            );

        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    list.push(value);
                }

                IterResult::End =>
                    break,
            }
        }

        Ok(
            Value::List(list)
        )
    }

    fn reduce_iterator(
        &mut self,
        iterator: IteratorRef,
        function: ClosureRef,
    ) -> Result<Value> {
        let first =
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) =>
                    value,

                IterResult::End =>
                    return Ok(
                        Value::Unit
                    ),
            };

        let mut accumulator =
            first;

        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    accumulator =
                        self.call_closure_sync(
                            function.clone(),
                            vec![
                                accumulator,
                                value,
                            ],
                        )?;
                }

                IterResult::End =>
                    return Ok(
                        accumulator
                    ),
            }
        }
    }

    fn fold_iterator(
        &mut self,
        iterator: IteratorRef,
        mut accumulator: Value,
        function: ClosureRef,
    ) -> Result<Value> {
        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    accumulator =
                        self.call_closure_sync(
                            function.clone(),
                            vec![
                                accumulator,
                                value,
                            ],
                        )?;
                }

                IterResult::End =>
                    return Ok(
                        accumulator
                    ),
            }
        }
    }

    fn any_iterator(
        &mut self,
        iterator: IteratorRef,
        predicate: ClosureRef,
    ) -> Result<Value> {
        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    let result =
                        self.call_closure_sync(
                            predicate.clone(),
                            vec![value],
                        )?;

                    match result {
                        Value::Bool(true) =>
                            return Ok(
                                Value::Bool(true)
                            ),

                        Value::Bool(false) => {}

                        other =>
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    format!(
                                        "any() predicate must return Bool, got {}",
                                        other.type_name()
                                    ),
                                    None,
                                )
                            ),
                    }
                }

                IterResult::End =>
                    return Ok(
                        Value::Bool(false)
                    ),
            }
        }
    }

    fn all_iterator(
        &mut self,
        iterator: IteratorRef,
        predicate: ClosureRef,
    ) -> Result<Value> {
        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    let result =
                        self.call_closure_sync(
                            predicate.clone(),
                            vec![value],
                        )?;

                    match result {
                        Value::Bool(false) =>
                            return Ok(
                                Value::Bool(false)
                            ),

                        Value::Bool(true) => {}

                        other =>
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    format!(
                                        "all() predicate must return Bool, got {}",
                                        other.type_name()
                                    ),
                                    None,
                                )
                            ),
                    }
                }

                IterResult::End =>
                    return Ok(
                        Value::Bool(true)
                    ),
            }
        }
    }

    fn numeric_reduce(
        &mut self,
        iterator: IteratorRef,
        op: BinOp,
    ) -> Result<Value> {
        let first =
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) =>
                    value,

                IterResult::End =>
                    return Ok(
                        Value::Int(
                            if op == BinOp::Mul {
                                1
                            } else {
                                0
                            }
                        )
                    ),
            };

        let mut accumulator =
            first;

        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    accumulator =
                        operator::apply_binop(
                            op,
                            accumulator,
                            value,
                        )
                        .map_err(|message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        })?;
                }

                IterResult::End =>
                    return Ok(
                        accumulator
                    ),
            }
        }
    }

    fn extreme_iterator(
        &mut self,
        iterator: IteratorRef,
        maximum: bool,
    ) -> Result<Value> {
        let first =
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) =>
                    value,

                IterResult::End =>
                    return Ok(
                        Value::Unit
                    ),
            };

        let mut extreme =
            first;

        loop {
            match self.iterator_next(
                iterator.clone()
            )? {
                IterResult::Item(value) => {
                    let op =
                        if maximum {
                            BinOp::Gt
                        } else {
                            BinOp::Lt
                        };

                    let greater =
                        operator::apply_binop(
                            op,
                            value.clone(),
                            extreme.clone(),
                        )
                        .map_err(|message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        })?;

                    let Value::Bool(
                        replace
                    ) = greater
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "comparison did not return Bool",
                                None,
                            )
                        );
                    };

                    if replace {
                        extreme =
                            value;
                    }
                }

                IterResult::End =>
                    return Ok(
                        extreme
                    ),
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
        string: Rc<String>,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "chars" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let iterator =
                    IteratorObj::Str {
                        data: string,
                        byte_index: 0,
                    };

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                iterator
                            )
                        )
                    )
                )
            }

            "len" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Int(
                        string.chars().count()
                            as i64
                    )
                )
            }

            "trim" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Str(
                        Rc::new(
                            string
                                .trim()
                                .to_owned()
                        )
                    )
                )
            }

            "to_upper" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Str(
                        Rc::new(
                            string
                                .to_uppercase()
                        )
                    )
                )
            }

            "to_lower" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Str(
                        Rc::new(
                            string
                                .to_lowercase()
                        )
                    )
                )
            }

            "contains" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    needle
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "contains() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                Ok(
                    Value::Bool(
                        string.contains(
                            needle.as_str()
                        )
                    )
                )
            }

            "starts_with" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    prefix
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "starts_with() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                Ok(
                    Value::Bool(
                        string.starts_with(
                            prefix.as_str()
                        )
                    )
                )
            }

            "ends_with" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    suffix
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "ends_with() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                Ok(
                    Value::Bool(
                        string.ends_with(
                            suffix.as_str()
                        )
                    )
                )
            }

            "split" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    separator
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "split() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                let values =
                    string
                        .split(
                            separator.as_str()
                        )
                        .map(|part| {
                            Value::Str(
                                Rc::new(
                                    part.to_owned()
                                )
                            )
                        })
                        .collect::<Vec<_>>();

                Ok(
                    Value::List(
                        Rc::new(
                            List::new(
                                values
                            )
                        )
                    )
                )
            }

            "replace" => {
                self.expect_arity(
                    name,
                    &args,
                    2,
                )?;

                let Value::Str(
                    from
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "replace() expects Str as first argument, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                let Value::Str(
                    to
                ) = &args[1]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "replace() expects Str as second argument, got {}",
                                args[1].type_name()
                            ),
                            None,
                        )
                    );
                };

                Ok(
                    Value::Str(
                        Rc::new(
                            string.replace(
                                from.as_str(),
                                to.as_str(),
                            )
                        )
                    )
                )
            }

            "repeat" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Int(
                    count
                ) = args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "repeat() expects Int, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                if count < 0 {
                    return Err(
                        Error::new(
                            ErrorKind::Value,
                            "repeat() does not accept negative counts",
                            None,
                        )
                    );
                }

                Ok(
                    Value::Str(
                        Rc::new(
                            string.repeat(
                                count as usize
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
                            "Str has no method '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn invoke_list_method(
        &mut self,
        list: ListRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "len" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Int(
                        list.len() as i64
                    )
                )
            }

            "push" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                list.push(
                    args[0].clone()
                );

                Ok(Value::Unit)
            }

            "iter" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

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
                    Value::Iterator(iterator)
                )
            }

            "map"
            | "filter"
            | "enumerate"
            | "zip"
            | "take"
            | "skip"
            | "collect"
            | "reduce"
            | "fold"
            | "any"
            | "all"
            | "sum"
            | "product"
            | "min"
            | "max" => {
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

                self.invoke_iterator_method(
                    iterator,
                    name,
                    args,
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
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                match self.iterator_next(
                    iterator
                )? {
                    IterResult::Item(value) =>
                        Ok(
                            Value::Tuple(
                                Rc::new(vec![
                                    value,
                                    Value::Bool(true),
                                ])
                            )
                        ),

                    IterResult::End =>
                        Ok(
                            Value::Tuple(
                                Rc::new(vec![
                                    Value::Unit,
                                    Value::Bool(false),
                                ])
                            )
                        ),
                }
            }

            "map" => {
                let closure =
                    Self::expect_closure_arg(
                        name,
                        &args,
                    )?;

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
                let closure =
                    Self::expect_closure_arg(
                        name,
                        &args,
                    )?;

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

            "enumerate" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Enumerate {
                                    source: iterator,
                                    index: 0,
                                }
                            )
                        )
                    )
                )
            }

            "zip" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let other =
                    match &args[0] {
                        Value::Iterator(
                            iterator
                        ) => iterator.clone(),

                        value => {
                            IteratorObj::from_value(
                                value.clone()
                            )
                            .map_err(|message| {
                                Error::new(
                                    ErrorKind::Type,
                                    message,
                                    None,
                                )
                            })?
                        }
                    };

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Zip {
                                    left: iterator,
                                    right: other,
                                }
                            )
                        )
                    )
                )
            }

            "take" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let count =
                    Self::expect_int_arg(
                        name,
                        &args[0],
                    )?;

                if count < 0 {
                    return Err(
                        Error::new(
                            ErrorKind::Value,
                            "take() count must be non-negative",
                            None,
                        )
                    );
                }

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Take {
                                    source: iterator,
                                    remaining:
                                        count as usize,
                                }
                            )
                        )
                    )
                )
            }

            "skip" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let count =
                    Self::expect_int_arg(
                        name,
                        &args[0],
                    )?;

                if count < 0 {
                    return Err(
                        Error::new(
                            ErrorKind::Value,
                            "skip() count must be non-negative",
                            None,
                        )
                    );
                }

                Ok(
                    Value::Iterator(
                        Rc::new(
                            RefCell::new(
                                IteratorObj::Skip {
                                    source: iterator,
                                    remaining:
                                        count as usize,
                                }
                            )
                        )
                    )
                )
            }

            "collect" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                self.collect_iterator(
                    iterator
                )
            }

            "reduce" => {
                let closure =
                    Self::expect_closure_arg(
                        name,
                        &args,
                    )?;

                self.reduce_iterator(
                    iterator,
                    closure,
                )
            }

            "fold" => {
                self.expect_arity(
                    name,
                    &args,
                    2,
                )?;

                let initial =
                    args[0].clone();

                let closure =
                    Self::expect_closure_arg_at(
                        name,
                        &args,
                        1,
                    )?;

                self.fold_iterator(
                    iterator,
                    initial,
                    closure,
                )
            }

            "any" => {
                let closure =
                    Self::expect_closure_arg(
                        name,
                        &args,
                    )?;

                self.any_iterator(
                    iterator,
                    closure,
                )
            }

            "all" => {
                let closure =
                    Self::expect_closure_arg(
                        name,
                        &args,
                    )?;

                self.all_iterator(
                    iterator,
                    closure,
                )
            }

            "sum" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                self.numeric_reduce(
                    iterator,
                    BinOp::Add,
                )
            }

            "product" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                self.numeric_reduce(
                    iterator,
                    BinOp::Mul,
                )
            }

            "min" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                self.extreme_iterator(
                    iterator,
                    false,
                )
            }

            "max" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                self.extreme_iterator(
                    iterator,
                    true,
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

        match name {
            "iter" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Iterator(
                        iterator
                    )
                )
            }

            _ => {
                self.invoke_iterator_method(
                    iterator,
                    name,
                    args,
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

    fn expect_arity(
        &self,
        name: &str,
        args: &[Value],
        expected: usize,
    ) -> Result<()> {
        if args.len() != expected {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{}() expects {} argument(s), got {}",
                        name,
                        expected,
                        args.len()
                    ),
                    None,
                )
            );
        }

        Ok(())
    }

    fn expect_int_arg(
        name: &str,
        value: &Value,
    ) -> Result<i64> {
        match value {
            Value::Int(value) =>
                Ok(*value),

            other =>
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "{}() expects Int, got {}",
                            name,
                            other.type_name()
                        ),
                        None,
                    )
                )
        }
    }

    fn expect_closure_arg(
        name: &str,
        args: &[Value],
    ) -> Result<ClosureRef> {
        Self::expect_closure_arg_at(
            name,
            args,
            0,
        )
    }

    fn expect_closure_arg_at(
        name: &str,
        args: &[Value],
        index: usize,
    ) -> Result<ClosureRef> {
        let value =
            args.get(index)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Arity,
                        format!(
                            "{}() missing closure argument",
                            name
                        ),
                        None,
                    )
                })?;

        match value {
            Value::Closure(
                closure
            ) =>
                Ok(closure.clone()),

            other =>
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "{}() expects a function, got {}",
                            name,
                            other.type_name()
                        ),
                        None,
                    )
                )
        }
    }

}