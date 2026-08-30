use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    }, 
    runtime::{
        List,
        Value,
        EnumRef,
        EnumValue,
        EnumConstructor,
        IteratorObj,
        IteratorRef,
        IterResult,
        StructValue,
        StructTypeRef,
        SeriesRef,
        DataFrameRef,
        CallFrame,
        RangeCursor,
        FunctionParameter,
        FunctionProto,
        FunctionRef,
        Closure,
        ClosureRef,
        UpvalueSpec,
        CellRef,
        Class,
        ClassRef,
        ObjectRef,
        FieldDefinition,
        Module,
        ModuleRef,
        ModulePath,
        apply_binop, 
    },
    syntax::BinOp,
    stdlib::{
        decode_class_counts,
        option_some,
        option_none,
    },
};

use super::{
    Chunk,
    OpCode,
    Instruction,
    PipelineState,
    PipelineStage,
    PipelineExpr,
    PipelinePlan,
    PipelineSource,
    PipelineProgram,
    IntPipelineExpr,
    IntPipelinePredicate,
    IntPipelineStage,
    ModuleLoader,
};

use std::{
    cell::RefCell, collections::HashMap, 
    path::{
        Path,
        PathBuf
    }, rc::Rc,
};

struct ExecutionResult {
    value: Value,
    frame: CallFrame,
}

pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,

    repl_locals: Vec<Value>,
    repl_cells: Vec<Option<CellRef>>,
    repl_module: ModuleRef,

    module_loader: ModuleLoader,
    modules: HashMap<PathBuf, ModuleRef>,
    loading_modules: Vec<PathBuf>,
    stdlib_modules: HashMap<String, ModuleRef>,
    module_namespaces: HashMap<ModulePath, ModuleRef>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(32),
            repl_locals: Vec::with_capacity(64),
            repl_cells: Vec::with_capacity(64),
            repl_module:
                Rc::new(
                    RefCell::new(
                        Module::new("<repl>")
                    )
                ),
            module_loader:
                ModuleLoader::new(
                    std::env::current_dir()
                        .expect(
                            "failed to get current directory"
                        )
                ),
            modules: HashMap::new(),
            loading_modules: Vec::new(),
            stdlib_modules: HashMap::new(),
            module_namespaces: HashMap::new(),
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
        let module =
            Rc::new(
                RefCell::new(
                    Module::new("<main>")
                )
            );

        self.run_with_module_and_path(
            chunk,
            module,
            None,
        )
    }

    pub fn run_with_module(
        &mut self,
        chunk: Rc<Chunk>,
        module: ModuleRef,
    ) -> Result<Value> {
        self.run_with_module_and_path(
            chunk,
            module,
            None,
        )
    }

    pub fn run_with_module_and_path(
        &mut self,
        chunk: Rc<Chunk>,
        module: ModuleRef,
        source_path: Option<&Path>,
    ) -> Result<Value> {
        self.stack.clear();
        self.frames.clear();

        let function =
            Rc::new(
                FunctionProto {
                    arity: 0,
                    parameters: Vec::new(),
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
                cells: None,
                range_cursors: Vec::new(),

                module: Some(module),

                source_path:
                    source_path
                        .map(Path::to_path_buf),
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
                    parameters: Vec::new(),
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

        let module =
            self.repl_module.clone();

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells: Some(cells),
                range_cursors: Vec::new(),
                module: Some(module),
                source_path: None,
            }
        );

        match self.execute_until_depth(0) {
            Ok(
                ExecutionResult {
                    value,
                    frame,
                }
            ) => {
                self.repl_locals =
                    frame.locals;

                self.repl_cells =
                    frame.cells
                        .unwrap_or_default();

                Ok(value)
            }

            Err(error) => {
                /*
                * Preserve the REPL frame even when execution fails.
                *
                * The current execution environment belongs to the
                * REPL and must survive ordinary expression errors.
                */
                if let Some(frame) =
                    self.frames.pop()
                {
                    self.repl_locals =
                        frame.locals;

                    self.repl_cells =
                        frame.cells
                            .unwrap_or_default();
                }

                Err(error)
            }
        }
    }

    fn execute(
        &mut self,
    ) -> Result<Value> {
        Ok(
            self.execute_until_depth(0)?
                .value
        )
    }

    fn execute_module(
        &mut self,
        chunk: Rc<Chunk>,
        module: ModuleRef,
        source_path: PathBuf,
    ) -> Result<Value> {
        let caller_depth =
            self.frames.len();

        let function =
            Rc::new(
                FunctionProto {
                    arity: 0,
                    parameters: Vec::new(),
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
                cells: None,
                range_cursors: Vec::new(),

                module:
                    Some(module),

                source_path:
                    Some(source_path),
            }
        );

        Ok(
            self.execute_until_depth(
                caller_depth
            )?
            .value
        )
    }

    fn execute_until_depth(
        &mut self,
        target_depth: usize,
    ) -> Result<ExecutionResult> {
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

                OpCode::JumpIfTrue => {
                    let condition =
                        self.pop_bool()?;

                    if condition {
                        self.current_frame_mut().ip =
                            operand as usize;
                    }
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

                    let value =
                        {
                            let frame =
                                self.current_frame();

                            match &frame.cells {
                                Some(cells) => {
                                    if let Some(
                                        Some(cell)
                                    ) =
                                        cells.get(slot)
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
                                    }
                                }

                                None => {
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
                                }
                            }
                        };

                    self.push(
                        value
                    );
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

                    if slot >=
                        frame.locals.len()
                    {
                        frame.locals.resize(
                            slot + 1,
                            Value::Unit,
                        );
                    }

                    if let Some(
                        cells
                    ) =
                        frame.cells.as_mut()
                    {
                        if cells.len() <= slot {
                            cells.resize(
                                slot + 1,
                                None,
                            );
                        }

                        if let Some(cell) =
                            cells[slot].clone()
                        {
                            *cell.borrow_mut() =
                                value.clone();
                        }
                    }

                    frame.locals[slot] =
                        value;
                }

                OpCode::ResetLocal => {
                    let slot =
                        operand as usize;

                    let frame =
                        self.current_frame_mut();

                    if slot >=
                        frame.locals.len()
                    {
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

                    if let Some(
                        cells
                    ) =
                        frame.cells.as_mut()
                    {
                        if cells.len() <= slot {
                            cells.resize(
                                slot + 1,
                                None,
                            );
                        }

                        cells[slot] =
                            None;
                    }

                    frame.locals[slot] =
                        Value::Unit;
                }

                OpCode::LoadBuiltin => {
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
                                    "builtin name constant out of bounds",
                                    None,
                                )
                            })?;

                    let Value::Str(name) =
                        value
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "LoadBuiltin requires Str constant",
                                None,
                            )
                        );
                    };

                    let builtin =
                        crate::stdlib::builtin::get(
                            name.as_str()
                        )
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                format!(
                                    "builtin '{}' is not registered",
                                    name
                                ),
                                None,
                            )
                        })?;

                    self.push(
                        builtin
                    );
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
                    let call_site =
                        operand as usize;

                    let metadata =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .call_sites
                            .get(call_site)
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "call-site index out of bounds",
                                    None,
                                )
                            })?;

                    let method_index =
                        metadata.method.ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                "InvokeMethod requires method call-site metadata",
                                None,
                            )
                        })?;

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
                        metadata.names.len();

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

                    let names =
                        metadata.names.clone();

                    let result =
                        self.invoke_method(
                            receiver,
                            method.as_str(),
                            args,
                            &names,
                        )?;

                    self.stack.truncate(
                        receiver_index
                    );

                    self.push(result);
                }

                OpCode::Call => {
                    let call_site =
                        operand as usize;

                    let metadata =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .call_sites
                            .get(call_site)
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "call-site index out of bounds",
                                    None,
                                )
                            })?;

                    let argc =
                        metadata.names.len();

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
                        ]
                        .clone();

                    let args =
                        self.stack[
                            function_index + 1..
                        ]
                        .to_vec();

                    let names =
                        metadata.names.clone();

                    let result =
                        self.call_value(
                            callable,
                            args,
                            &names,
                        )?;

                    self.stack.truncate(
                        function_index
                    );

                    self.push(result);
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
                        List::with_capacity(
                            operand as usize
                        );

                    self.push(
                        Value::List(
                            list
                        )
                    );
                }

                OpCode::NewDict => {
                    self.push(
                        Value::Dict(
                            Rc::new(
                                RefCell::new(
                                    HashMap::with_capacity(
                                        operand as usize
                                    )
                                )
                            )
                        )
                    );
                }

                OpCode::NewClass => {
                    let (
                        field_count,
                        method_count,
                    ) =
                        decode_class_counts(
                            operand
                        );

                    let required =
                        1
                        + field_count * 2
                        + method_count * 2;

                    if self.stack.len() < required {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "VM stack underflow while creating class",
                                None,
                            )
                        );
                    }

                    let mut method_values =
                        Vec::with_capacity(
                            method_count
                        );

                    for _ in 0..method_count {
                        let closure =
                            self.pop()?;

                        let name =
                            self.pop()?;

                        let (
                            name,
                            closure,
                        ) =
                            match (name, closure) {
                                (
                                    Value::Str(name),
                                    Value::Closure(closure),
                                ) => (
                                    name,
                                    closure,
                                ),

                                _ => {
                                    return Err(
                                        Error::new(
                                            ErrorKind::Runtime,
                                            "invalid class method descriptor",
                                            None,
                                        )
                                    );
                                }
                            };

                        method_values.push(
                            (
                                name.to_string(),
                                closure,
                            )
                        );
                    }

                    let mut field_values =
                        Vec::with_capacity(
                            field_count
                        );

                    for _ in 0..field_count {
                        let default =
                            self.pop()?;

                        let name =
                            self.pop()?;

                        let name =
                            match name {
                                Value::Str(name) =>
                                    name,

                                _ => {
                                    return Err(
                                        Error::new(
                                            ErrorKind::Runtime,
                                            "invalid class field descriptor",
                                            None,
                                        )
                                    );
                                }
                            };

                        let default =
                            match default {
                                Value::Closure(closure) =>
                                    Some(closure),

                                Value::Unit =>
                                    None,

                                _ => {
                                    return Err(
                                        Error::new(
                                            ErrorKind::Runtime,
                                            "invalid class field default",
                                            None,
                                        )
                                    );
                                }
                            };

                        field_values.push(
                            FieldDefinition::new(
                                name.as_str(),
                                default,
                            )
                        );
                    }

                    let class_name =
                        match self.pop()? {
                            Value::Str(name) =>
                                name,

                            _ => {
                                return Err(
                                    Error::new(
                                        ErrorKind::Runtime,
                                        "class name must be Str",
                                        None,
                                    )
                                );
                            }
                        };

                    let methods:
                        HashMap<String, ClosureRef> =
                        method_values
                            .into_iter()
                            .collect();

                    let class =
                        Class::new(
                            class_name.as_str(),
                            field_values,
                            methods,
                        );

                    self.push(
                        Value::Class(
                            Rc::new(class)
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

                        (
                            Value::Series(series),
                            Value::Int(index),
                        ) => {
                            if index < 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Index,
                                        "Series index must be non-negative",
                                        None,
                                    )
                                );
                            }

                            let value =
                                series
                                    .get(index as usize)
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            "series index out of bounds",
                                            None,
                                        )
                                    })?;

                            self.push(value);
                        }

                        (
                            Value::DataFrame(df),
                            Value::Str(name),
                        ) => {
                            let column =
                                df.column(name.as_str())
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            format!(
                                                "DataFrame column not found: '{}'",
                                                name
                                            ),
                                            None,
                                        )
                                    })?;

                            self.push(
                                Value::Series(column)
                            );
                        }

                        (
                            Value::DataFrame(df),
                            Value::Int(index),
                        ) => {
                            if index < 0 {
                                return Err(
                                    Error::new(
                                        ErrorKind::Index,
                                        "DataFrame row index must be non-negative",
                                        None,
                                    )
                                );
                            }

                            let row =
                                df.row(index as usize)
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            "DataFrame row index out of bounds",
                                            None,
                                        )
                                    })?;

                            self.push(
                                Value::Dict(row)
                            );
                        }

                        (
                            Value::Dict(dict),
                            Value::Str(key),
                        ) => {
                            let value =
                                dict.borrow()
                                    .get(key.as_str())
                                    .cloned()
                                    .ok_or_else(|| {
                                        Error::new(
                                            ErrorKind::Index,
                                            format!(
                                                "dict key not found: '{}'",
                                                key,
                                            ),
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
                    /*
                    * Stack contract:
                    *
                    *     [object, index, value]
                    *
                    * IndexSet must not mutate the stack until the assignment
                    * itself has succeeded. This guarantees that a failed
                    * indexed assignment does not corrupt the VM stack.
                    */

                    if self.stack.len() < 3 {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "indexed assignment stack underflow",
                                None,
                            )
                        );
                    }

                    let object_index =
                        self.stack.len() - 3;

                    let index_index =
                        self.stack.len() - 2;

                    let value_index =
                        self.stack.len() - 1;

                    let object =
                        self.stack[
                            object_index
                        ]
                        .clone();

                    let index =
                        self.stack[
                            index_index
                        ]
                        .clone();

                    let value =
                        self.stack[
                            value_index
                        ]
                        .clone();

                    match (
                        object,
                        index,
                    ) {
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

                            /*
                            * Perform the fallible mutation first.
                            * The stack remains unchanged if set() fails.
                            */
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

                            /*
                            * Assignment succeeded.
                            *
                            * [object, index, value]
                            *        ↓
                            *        [value]
                            */
                            self.stack.truncate(
                                object_index
                            );

                            self.push(
                                value
                            );
                        }

                        (
                            Value::Dict(dict),
                            Value::Str(key),
                        ) => {
                            /*
                            * HashMap::insert() itself cannot fail here,
                            * so the mutation can be committed directly.
                            */
                            dict.borrow_mut().insert(
                                key.as_str().to_owned(),
                                value.clone(),
                            );

                            /*
                            * [object, index, value]
                            *        ↓
                            *        [value]
                            */
                            self.stack.truncate(
                                object_index
                            );

                            self.push(
                                value
                            );
                        }

                        (
                            _,
                            _,
                        ) => {
                            /*
                            * No stack mutation has occurred.
                            */
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
                    if self.stack.len() < 2 {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "field access stack underflow",
                                None,
                            )
                        );
                    }

                    let field =
                        match self.stack[
                            self.stack.len() - 1
                        ].clone() {
                            Value::Str(field) =>
                                field,

                            _ => {
                                return Err(
                                    Error::new(
                                        ErrorKind::Type,
                                        "field name must be Str",
                                        None,
                                    )
                                );
                            }
                        };

                    let object =
                        self.stack[
                            self.stack.len() - 2
                        ].clone();

                    let value =
                        self.resolve_field(
                            object,
                            field.as_str(),
                        )?;

                    self.stack.truncate(
                        self.stack.len() - 2
                    );

                    self.push(value);
                }

                OpCode::FieldSet => {
                    let field =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    let value =
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
                        Value::Object(object) => {
                            let class = {
                                let object_ref =
                                    object.borrow();

                                object_ref.class()
                            };

                            if class
                                .field(
                                    field.as_str()
                                )
                                .is_none()
                            {
                                return Err(
                                    Error::new(
                                        ErrorKind::Name,
                                        format!(
                                            "class '{}' has no field '{}'",
                                            class.name(),
                                            field,
                                        ),
                                        None,
                                    )
                                );
                            }

                            object
                                .borrow_mut()
                                .set_field(
                                    field.as_str(),
                                    value.clone(),
                                );

                            self.push(value);
                        }

                        Value::Struct(_) => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "struct fields are immutable",
                                    None,
                                )
                            );
                        }

                        _ => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "field assignment requires an object",
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

                OpCode::StructFieldGet => {
                    let field =
                        self.pop()?;

                    let object =
                        self.pop()?;

                    let Value::Str(field) =
                        field
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "struct field name must be Str",
                                None,
                            )
                        );
                    };

                    let Value::Struct(value) =
                        object
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "StructFieldGet expects Struct",
                                None,
                            )
                        );
                    };

                    let result =
                        value
                            .get_field(field.as_str())
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Name,
                                    format!(
                                        "{} has no field '{}'",
                                        value.type_name(),
                                        field,
                                    ),
                                    None,
                                )
                            })?;

                    self.push(result);
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

                OpCode::RangeInit => {
                    let range_index =
                        operand as usize;

                    let range =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .range_loops
                            .get(range_index)
                            .copied()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "range loop index out of bounds",
                                    None,
                                )
                            })?;

                    /*
                    * Stack:
                    *
                    *     [start, end]
                    */
                    let end =
                        self.pop()?;

                    let start =
                        self.pop()?;

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

                    /*
                    * Normalize inclusive ranges once.
                    *
                    *     [start, end]
                    *          =>
                    *     [start, end + 1)
                    */
                    let exclusive_end =
                        if range.inclusive {
                            end.checked_add(1)
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::Overflow,
                                        "inclusive range endpoint overflow",
                                        None,
                                    )
                                })?
                        } else {
                            end
                        };

                    let frame =
                        self.current_frame_mut();

                    if frame.range_cursors.len()
                        <= range_index
                    {
                        frame.range_cursors.resize(
                            range_index + 1,
                            None,
                        );
                    }

                    frame.range_cursors[
                        range_index
                    ] =
                        Some(
                            RangeCursor {
                                current: start,
                                end: exclusive_end,
                            }
                        );
                }

                OpCode::RangeNext => {
                    self.advance_range(
                        operand as usize
                    )?;
                }

                OpCode::FusedPipeline => {
                    self.execute_fused_pipeline(
                        operand as usize
                    )?;
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

                    let expected_arity =
                        operand as usize;

                    let matched =
                        match &value {
                            Value::EnumValue(
                                enum_value
                            ) => {
                                enum_value
                                    .enum_name()
                                    == enum_name.as_str()
                                &&
                                enum_value
                                    .variant()
                                    == variant.as_str()
                                &&
                                enum_value
                                    .fields()
                                    .len()
                                    == expected_arity
                            }

                            _ => false,
                        };

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

                    self.push(
                        Value::Bool(matched)
                    );
                }

                OpCode::MatchStruct => {
                    let name =
                        self.pop()?;

                    let value =
                        self.pop()?;

                    let Value::Str(name) =
                        name
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Type,
                                "MatchStruct expects struct name",
                                None,
                            )
                        );
                    };

                    let matched =
                        match &value {
                            Value::Struct(value) => {
                                let fields =
                                    value.fields();

                                value.type_name()
                                    == name.as_str()
                                    && fields.len()
                                        >= operand as usize
                            }

                            _ => false,
                        };

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

                OpCode::LoadModule => {
                    let reference =
                        self.current_frame()
                            .closure
                            .function
                            .chunk
                            .module_refs
                            .get(
                                operand as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Import,
                                    "module reference index out of bounds",
                                    None,
                                )
                            })?;

                    let module =
                        if reference.namespace {
                            self.load_module_namespace(
                                &reference.path
                            )?
                        } else {
                            self.load_module(
                                &reference.path
                            )?
                        };

                    self.push(
                        Value::Module(module)
                    );
                }

                OpCode::Export => {
                    let name =
                        self.pop()?;

                    let Value::Str(name) =
                        name
                    else {
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "export name must be Str",
                                None,
                            )
                        );
                    };

                    let value =
                        self.stack
                            .last()
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "VM stack underflow while exporting",
                                    None,
                                )
                            })?;

                    let module =
                        self.current_frame()
                            .module
                            .clone()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    "no active module",
                                    None,
                                )
                            })?;

                    module
                        .borrow_mut()
                        .set_exported(
                            name.as_str(),
                            value,
                        );
                }

                OpCode::Return => {
                    let result =
                        self.pop()?;

                    if let Some(execution) =
                        self.return_from_current_frame(
                            result,
                            target_depth,
                        )?
                    {
                        return Ok(execution);
                    }
                }

                OpCode::Try => {
                    let value =
                        self.pop()?;

                    if let Some(execution) =
                        self.try_value(
                            value,
                            target_depth,
                        )?
                    {
                        return Ok(execution);
                    }
                }

                OpCode::Halt => {
                    let result =
                        self.pop()?;

                    let frame =
                        self.frames
                            .pop()
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
                        return Ok(
                            ExecutionResult {
                                value: result,
                                frame,
                            }
                        );
                    }

                    self.push(result);
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

    fn execute_fused_pipeline(
        &mut self,
        pipeline_index: usize,
    ) -> Result<()> {
        let pipeline =
            self.current_frame()
                .closure
                .function
                .chunk
                .pipelines
                .get(pipeline_index)
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "pipeline index out of bounds",
                        None,
                    )
                })?;

        /*
        * Resolve source exactly once.
        */
        let (
            start,
            end,
            inclusive,
        ) =
            self.resolve_pipeline_source(
                &pipeline.source
            )?;

        /*
        * Now the runtime capacity is known.
        */
        let capacity =
            PipelineProgram::
                capacity_upper_bound_for_range(
                    start,
                    end,
                    inclusive,
                    &pipeline.stages,
                )?;

        let list =
            List::with_capacity(
                capacity
            );

        let stage_captures =
            self.resolve_pipeline_plan_captures(
                &pipeline.stages
            )?;

        match &pipeline.plan {
            PipelinePlan::IntRange {
                stages,
            } => {
                self.execute_int_range_pipeline(
                    &pipeline,
                    start,
                    end,
                    inclusive,
                    stages,
                    &stage_captures,
                    &list,
                )?;
            }

            PipelinePlan::Generic => {
                self.execute_fused_range_pipeline(
                    start,
                    end,
                    inclusive,
                    &pipeline.stages,
                    &stage_captures,
                    &list,
                )?;
            }
        }

        self.push(
            Value::List(list)
        );

        Ok(())
    }

    fn execute_fused_range_pipeline(
        &mut self,
        start: i64,
        end: i64,
        inclusive: bool,
        stages: &[PipelineStage],
        stage_captures: &[Vec<CellRef>],
        output: &List,
    ) -> Result<()> {
        if stage_captures.len() !=
            stages.len()
        {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline capture metadata does not match pipeline stages",
                    None,
                )
            );
        }
        
        /*
        * Normalize inclusive ranges once.
        *
        * [start, end]
        * becomes
        * [start, end + 1)
        */
        let end =
            if inclusive {
                end.checked_add(1)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Overflow,
                            "inclusive range endpoint overflow",
                            None,
                        )
                    })?
            } else {
                end
            };

        /*
        * Empty range.
        */
        if start >= end {
            return Ok(());
        }

        /*
        * Stateful stages keep their runtime state here.
        *
        * Map / Filter:
        *     None
        *
        * Skip(n):
        *     Skip(n)
        *
        * Take(n):
        *     Take(n)
        */
        let mut states =
            stages
                .iter()
                .map(
                    |stage| {
                        match stage {
                            PipelineStage::Skip {
                                count,
                            } =>
                                PipelineState::Skip(
                                    *count
                                ),

                            PipelineStage::Take {
                                count,
                            } =>
                                PipelineState::Take(
                                    *count
                                ),

                            PipelineStage::Map {
                                ..
                            }
                            |
                            PipelineStage::Filter {
                                ..
                            } =>
                                PipelineState::None,
                        }
                    }
                )
                .collect::<Vec<_>>();

        /*
        * Main fused loop.
        *
        * No IteratorObj::Range is created here.
        * No IteratorNext is required.
        *
        * The range itself is the source iterator.
        */
        for current in start..end {
            /*
            * The current item is represented as a Value only once
            * at the beginning of the iteration.
            */
            let mut value =
                Value::Int(
                    current
                );

            let mut accepted =
                true;

            /*
            * Execute every pipeline stage in source order.
            */
            for (
                index,
                stage,
            ) in stages.iter().enumerate()
            {
                match stage {
                    /*
                    * --------------------------------------------
                    * map
                    * --------------------------------------------
                    */
                    PipelineStage::Map {
                        expr,
                        ..
                    } => {
                        value =
                            self.eval_pipeline_expr(
                                expr,
                                value,
                                &stage_captures[index],
                            )?;
                    }

                    /*
                    * --------------------------------------------
                    * filter
                    * --------------------------------------------
                    */
                    PipelineStage::Filter {
                        expr,
                        ..
                    } => {
                        let predicate =
                            self.eval_pipeline_expr(
                                expr,
                                value.clone(),
                                &stage_captures[index],
                            )?;

                        let Value::Bool(
                            keep
                        ) = predicate
                        else {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    "filter predicate must return Bool",
                                    None,
                                )
                            );
                        };

                        if !keep {
                            accepted = false;
                            break;
                        }
                    }

                    /*
                    * --------------------------------------------
                    * skip
                    * --------------------------------------------
                    */
                    PipelineStage::Skip {
                        ..
                    } => {
                        let PipelineState::Skip(
                            remaining
                        ) =
                            &mut states[index]
                        else {
                            unreachable!(
                                "pipeline state mismatch for Skip"
                            );
                        };

                        if *remaining > 0 {
                            *remaining -= 1;

                            /*
                            * Skip consumes the item.
                            */
                            accepted = false;
                            break;
                        }
                    }

                    /*
                    * --------------------------------------------
                    * take
                    * --------------------------------------------
                    */
                    PipelineStage::Take {
                        ..
                    } => {
                        let PipelineState::Take(
                            remaining
                        ) =
                            &mut states[index]
                        else {
                            unreachable!(
                                "pipeline state mismatch for Take"
                            );
                        };

                        /*
                        * If this Take stage has already exhausted
                        * its quota, the entire lazy pipeline is done.
                        */
                        if *remaining == 0 {
                            return Ok(());
                        }

                        /*
                        * This source item is consumed by Take.
                        */
                        *remaining -= 1;
                    }
                }
            }

            /*
            * Only accepted values reach collect().
            */
            if accepted {
                output.push(
                    value
                );
            }
        }

        Ok(())
    }

    fn execute_int_range_pipeline(
        &mut self,
        _pipeline: &PipelineProgram,
        start: i64,
        end: i64,
        inclusive: bool,
        stages: &[IntPipelineStage],
        stage_captures: &[Vec<CellRef>],
        output: &List,
    ) -> Result<()> {
        if stage_captures.len() !=
            stages.len()
        {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "pipeline capture metadata does not match pipeline stages",
                    None,
                )
            );
        }

        let end =
            if inclusive {
                end.checked_add(1)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Overflow,
                            "inclusive range endpoint overflow",
                            None,
                        )
                    })?
            } else {
                end
            };

        if start >= end {
            return Ok(());
        }

        /*
        * Stateful stages are kept separately from the immutable
        * stage description. Stage order itself is never changed.
        */
        let mut skip_remaining =
            vec![0usize; stages.len()];

        let mut take_remaining:
            Vec<Option<usize>> =
            vec![None; stages.len()];

        for (
            index,
            stage,
        ) in stages.iter().enumerate()
        {
            match stage {
                IntPipelineStage::Skip(
                    count
                ) => {
                    skip_remaining[index] =
                        *count;
                }

                IntPipelineStage::Take(
                    count
                ) => {
                    take_remaining[index] =
                        Some(*count);
                }

                IntPipelineStage::Map(_)
                |
                IntPipelineStage::Filter(_) => {}
            }
        }

        /*
        * Fused integer range loop.
        *
        * `value` remains an i64 for the entire pipeline.
        * A Value::Int is created only when an item is finally
        * emitted into the output List.
        */
        for current in start..end {
            let mut value =
                current;

            let mut accepted =
                true;

            for (
                index,
                stage,
            ) in stages.iter().enumerate()
            {
                match stage {
                    IntPipelineStage::Map(
                        expr
                    ) => {
                        value =
                            self.eval_int_pipeline_expr(
                                expr,
                                value,
                                &stage_captures[index],
                            )?;
                    }

                    IntPipelineStage::Filter(
                        predicate
                    ) => {
                        let keep =
                            self.eval_int_pipeline_predicate(
                                predicate,
                                value,
                                &stage_captures[index],
                            )?;

                        if !keep {
                            accepted =
                                false;

                            break;
                        }
                    }

                    IntPipelineStage::Skip(
                        _
                    ) => {
                        if skip_remaining[index] > 0 {
                            skip_remaining[index] -= 1;

                            accepted =
                                false;

                            break;
                        }
                    }

                    IntPipelineStage::Take(
                        _
                    ) => {
                        let remaining =
                            take_remaining[index]
                                .as_mut()
                                .expect(
                                    "missing Take runtime state"
                                );

                        /*
                        * Take is exhausted: the entire lazy pipeline
                        * is exhausted, regardless of stages following
                        * this Take.
                        */
                        if *remaining == 0 {
                            return Ok(());
                        }

                        /*
                        * This value passed through the Take stage.
                        */
                        *remaining -= 1;
                    }
                }
            }

            if accepted {
                output.push(
                    Value::Int(
                        value
                    )
                );
            }
        }

        Ok(())
    }

    fn get_series_property(
        &self,
        series: SeriesRef,
        name: &str,
    ) -> Result<Value> {
        match name {
            "name" => {
                Ok(
                    Value::Str(
                        Rc::new(
                            series
                                .name()
                                .to_owned()
                        )
                    )
                )
            }

            "len" => {
                Ok(
                    Value::Int(
                        series.len() as i64
                    )
                )
            }

            "is_empty" => {
                Ok(
                    Value::Bool(
                        series.is_empty()
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "Series has no property '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn get_dataframe_property(
        &self,
        df: DataFrameRef,
        name: &str,
    ) -> Result<Value> {
        match name {
            "nrows" => {
                Ok(
                    Value::Int(
                        df.nrows() as i64
                    )
                )
            }

            "ncols" => {
                Ok(
                    Value::Int(
                        df.ncols() as i64
                    )
                )
            }

            "columns" => {
                let values =
                    df.columns()
                        .into_iter()
                        .map(|name| {
                            Value::Str(
                                Rc::new(name)
                            )
                        })
                        .collect();

                Ok(
                    Value::List(
                        List::new(values)
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "DataFrame has no property '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn resolve_field(
        &self,
        object: Value,
        field: &str,
    ) -> Result<Value> {
        match object {
            Value::Enum(enum_def) => {
                let variant =
                    enum_def
                        .variant(field)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Name,
                                format!(
                                    "enum '{}' has no variant '{}'",
                                    enum_def.name(),
                                    field,
                                ),
                                None,
                            )
                        })?;

                let arity =
                    variant.arity();

                if arity == 0 {
                    Ok(
                        Value::EnumValue(
                            Rc::new(
                                EnumValue::new(
                                    enum_def.name(),
                                    field,
                                    Vec::new(),
                                )
                            )
                        )
                    )
                } else {
                    Ok(
                        Value::EnumConstructor(
                            EnumConstructor::new(
                                enum_def,
                                field,
                            )
                        )
                    )
                }
            }

            Value::Struct(value) => {
                value
                    .get_field(field)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "{} has no field '{}'",
                                value.type_name(),
                                field,
                            ),
                            None,
                        )
                    })
            }

            Value::Object(object) => {
                let object_ref =
                    object.borrow();

                object_ref
                    .get_field(field)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "{} has no field '{}'",
                                object_ref.type_name(),
                                field,
                            ),
                            None,
                        )
                    })
            }

            Value::Module(module) => {
                let module_ref =
                    module.borrow();

                module_ref
                    .get_field(field)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "{} has no field '{}'",
                                module_ref.name(),
                                field,
                            ),
                            None,
                        )
                    })
            }

            Value::Series(series) => {
                self.get_series_property(
                    series,
                    field,
                )
            }

            Value::DataFrame(df) => {
                self.get_dataframe_property(
                    df,
                    field,
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Type,
                        "unsupported field access",
                        None,
                    )
                )
            }
        }
    }

    fn resolve_pipeline_source(
        &self,
        source: &PipelineSource,
    ) -> Result<(i64, i64, bool)> {
        match source {
            PipelineSource::Range {
                start,
                end,
                inclusive,
            } => {
                Ok((
                    *start,
                    *end,
                    *inclusive,
                ))
            }

            PipelineSource::DynamicRange {
                start,
                end,
                inclusive,
                captures,
                require_non_negative_end,
            } => {
                let captures =
                    self.resolve_pipeline_captures(
                        captures
                    )?;

                let start =
                    self.eval_pipeline_expr(
                        start,
                        Value::Unit,
                        &captures,
                    )?;

                let end =
                    self.eval_pipeline_expr(
                        end,
                        Value::Unit,
                        &captures,
                    )?;

                let start =
                    match start {
                        Value::Int(value) =>
                            value,

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
                        Value::Int(value) =>
                            value,

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

                if *require_non_negative_end
                    && end < 0
                {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            "range() requires a non-negative argument",
                            None,
                        )
                    );
                }

                Ok((
                    start,
                    end,
                    *inclusive,
                ))
            }
        }
    }

    fn resolve_pipeline_captures(
        &self,
        specs: &[UpvalueSpec],
    ) -> Result<Vec<CellRef>> {
        let frame =
            self.current_frame();

        let mut captures =
            Vec::with_capacity(
                specs.len()
            );

        for spec in specs {
            match spec {
                UpvalueSpec::Local(
                    slot
                ) => {
                    if let Some(
                        cells
                    ) =
                        frame.cells.as_ref()
                    {
                        if let Some(
                            Some(cell)
                        ) =
                            cells.get(
                                *slot as usize
                            )
                        {
                            captures.push(
                                cell.clone()
                            );

                            continue;
                        }
                    }

                    let value =
                        frame
                            .locals
                            .get(
                                *slot as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    format!(
                                        "pipeline capture local slot out of bounds: {}",
                                        slot
                                    ),
                                    None,
                                )
                            })?;

                    captures.push(
                        Rc::new(
                            RefCell::new(
                                value
                            )
                        )
                    );
                }

                UpvalueSpec::Parent(
                    slot
                ) => {
                    let cell =
                        frame
                            .closure
                            .upvalues
                            .get(
                                *slot as usize
                            )
                            .cloned()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    format!(
                                        "pipeline parent upvalue slot out of bounds: {}",
                                        slot
                                    ),
                                    None,
                                )
                            })?;

                    captures.push(
                        cell
                    );
                }
            }
        }

        Ok(captures)
    }

    fn resolve_pipeline_plan_captures(
        &self,
        stages: &[PipelineStage],
    ) -> Result<Vec<Vec<CellRef>>> {
        let mut result =
            Vec::with_capacity(
                stages.len()
            );

        for stage in stages {
            match stage {
                PipelineStage::Map {
                    captures,
                    ..
                }
                |
                PipelineStage::Filter {
                    captures,
                    ..
                } => {
                    result.push(
                        self.resolve_pipeline_captures(
                            captures
                        )?
                    );
                }

                PipelineStage::Skip { .. }
                |
                PipelineStage::Take { .. } => {
                    result.push(
                        Vec::new()
                    );
                }
            }
        }

        Ok(result)
    }

    fn return_from_current_frame(
        &mut self,
        value: Value,
        target_depth: usize,
    ) -> Result<Option<ExecutionResult>> {
        let frame =
            self.frames
                .pop()
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
            return Ok(
                Some(
                    ExecutionResult {
                        value,
                        frame,
                    }
                )
            );
        }

        self.push(value);

        Ok(None)
    }

    fn bind_arguments(
        &self,
        parameters: &[FunctionParameter],
        names: &[Option<String>],
        values: Vec<Value>,
    ) -> Result<Vec<Value>> {
        if names.len() != values.len() {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "call-site metadata does not match arguments",
                    None,
                )
            );
        }

        if values.len() >
            parameters.len()
        {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "function expects at most {} arguments, got {}",
                        parameters.len(),
                        values.len()
                    ),
                    None,
                )
            );
        }

        let mut bound =
            vec![
                Value::Unit;
                parameters.len()
            ];

        let mut assigned =
            vec![
                false;
                parameters.len()
            ];

        let mut positional_index =
            0usize;

        for (
            name,
            value,
        ) in names
            .iter()
            .zip(values)
        {
            match name {
                None => {
                    while positional_index <
                        assigned.len()
                        &&
                        assigned[
                            positional_index
                        ]
                    {
                        positional_index += 1;
                    }

                    if positional_index >=
                        parameters.len()
                    {
                        return Err(
                            Error::new(
                                ErrorKind::Arity,
                                "too many positional arguments",
                                None,
                            )
                        );
                    }

                    bound[
                        positional_index
                    ] = value;

                    assigned[
                        positional_index
                    ] = true;

                    positional_index += 1;
                }

                Some(name) => {
                    let index =
                        parameters
                            .iter()
                            .position(
                                |parameter| {
                                    parameter.name.as_deref()
                                        == Some(
                                            name.as_str()
                                        )
                                }
                            )
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Name,
                                    format!(
                                        "unknown parameter '{}'",
                                        name
                                    ),
                                    None,
                                )
                            })?;

                    if assigned[index] {
                        return Err(
                            Error::new(
                                ErrorKind::Arity,
                                format!(
                                    "argument '{}' specified more than once",
                                    name
                                ),
                                None,
                            )
                        );
                    }

                    bound[index] =
                        value;

                    assigned[index] =
                        true;
                }
            }
        }

        if assigned
            .iter()
            .any(|assigned| !assigned)
        {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    "missing required argument",
                    None,
                )
            );
        }

        /*
        * A destructured parameter has no name, so it can only
        * be supplied positionally.
        */
        Ok(bound)
    }

    fn call_closure_sync(
        &mut self,
        closure: ClosureRef,
        args: Vec<Value>,
    ) -> Result<Value> {
        let names =
            vec![None; args.len()];

        self.call_closure_sync_named(
            closure,
            args,
            &names,
        )
    }

    fn call_closure_sync1(
        &mut self,
        closure: ClosureRef,
        arg: Value,
    ) -> Result<Value> {
        let expected =
            closure.function.arity as usize;

        if expected != 1 {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "function expects {} arguments, got 1",
                        expected
                    ),
                    None,
                )
            );
        }

        let caller_depth =
            self.frames.len();

        let module =
            self.current_frame()
                .module
                .clone();

        let source_path =
            self.current_frame()
                .source_path
                .clone();

        let local_count =
            closure
                .function
                .chunk
                .local_count;

        let mut locals =
            Vec::with_capacity(
                local_count
            );

        locals.push(arg);

        if local_count > 1 {
            locals.resize(
                local_count,
                Value::Unit,
            );
        }

        let cells =
            vec![
                None;
                local_count
            ];

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells: Some(cells),
                range_cursors: Vec::new(),

                module,
                source_path,
            }
        );

        Ok(
            self.execute_until_depth(
                caller_depth
            )?
            .value
        )
    }

    /// For `reduce()` and `fold()`
    fn call_closure_sync2(
        &mut self,
        closure: ClosureRef,
        arg0: Value,
        arg1: Value,
    ) -> Result<Value> {
        let expected =
            closure.function.arity as usize;

        if expected != 2 {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "function expects {} arguments, got 2",
                        expected
                    ),
                    None,
                )
            );
        }

        let caller_depth =
            self.frames.len();

        let local_count =
            closure
                .function
                .chunk
                .local_count;

        let mut locals =
            Vec::with_capacity(
                local_count
            );

        locals.push(
            arg0
        );

        locals.push(
            arg1
        );

        if local_count > 2 {
            locals.resize(
                local_count,
                Value::Unit,
            );
        }

        let cells =
            vec![
                None;
                local_count
            ];

        let module =
            self.current_frame()
                .module
                .clone();

        let source_path =
            self.current_frame()
                .source_path
                .clone();

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells: Some(cells),
                range_cursors: Vec::new(),

                module,
                source_path,
            }
        );

        Ok(
            self.execute_until_depth(
                caller_depth
            )?
            .value
        )
    }

    fn call_closure_sync_named(
        &mut self,
        closure: ClosureRef,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        let bound =
            self.bind_arguments(
                &closure.function.parameters,
                names,
                args,
            )?;

        let caller_depth =
            self.frames.len();

        let local_count =
            closure
                .function
                .chunk
                .local_count;

        let mut locals =
            bound;

        locals.resize(
            local_count,
            Value::Unit,
        );

        let module =
            self.current_frame()
                .module
                .clone();

        let source_path =
            self.current_frame()
                .source_path
                .clone();

        self.frames.push(
            CallFrame {
                closure,
                ip: 0,
                locals,
                cells:
                    vec![
                        None;
                        local_count
                    ].into(),
                range_cursors: Vec::new(),

                module,
                source_path,
            }
        );

        Ok(
            self.execute_until_depth(
                caller_depth
            )?
            .value
        )
    }

    fn call_class_value(
        &mut self,
        class: ClassRef,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        if names.len() != args.len() {
            return Err(
                Error::new(
                    ErrorKind::Runtime,
                    "class call-site metadata does not match arguments",
                    None,
                )
            );
        }

        let object =
            class.instantiate();

        /*
        * Initialize instance fields.
        */
        for field in class.fields() {
            let value =
                match field.default() {
                    Some(closure) =>
                        self.call_closure_sync(
                            closure,
                            Vec::new(),
                        )?,

                    None =>
                        Value::Unit,
                };

            object
                .borrow_mut()
                .set_field(
                    field.name(),
                    value,
                );
        }

        /*
        * Run init(self, ...).
        *
        * `self` is an implicit positional argument.
        * User arguments keep their original named metadata.
        */
        if let Some(constructor) =
            class.constructor()
        {
            let mut call_args =
                Vec::with_capacity(
                    args.len() + 1
                );

            let mut call_names =
                Vec::with_capacity(
                    names.len() + 1
                );

            call_args.push(
                Value::Object(
                    object.clone()
                )
            );

            call_names.push(
                None
            );

            call_args.extend(
                args
            );

            call_names.extend(
                names.iter().cloned()
            );

            self.call_closure_sync_named(
                constructor,
                call_args,
                &call_names,
            )?;
        } else if !args.is_empty() {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{}() expects 0 arguments, got {}",
                        class.name(),
                        args.len(),
                    ),
                    None,
                )
            );
        }

        Ok(
            Value::Object(
                object
            )
        )
    }

    fn call_value(
        &mut self,
        callable: Value,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        match callable {
            Value::Closure(
                closure
            ) => {
                self.call_closure_sync_named(
                    closure,
                    args,
                    names,
                )
            }

            Value::Builtin(
                builtin
            ) => {
                ensure_positional_args(
                    names
                )?;

                builtin(args)
                    .map_err(|message| {
                        Error::new(
                            ErrorKind::Runtime,
                            message,
                            None,
                        )
                    })
            }

            Value::Class(
                class
            ) => {
                self.call_class_value(
                    class,
                    args,
                    names,
                )
            }

            Value::EnumConstructor(
                constructor
            ) => {
                ensure_positional_args(
                    names
                )?;

                self.make_enum_value(
                    constructor,
                    args,
                )
            }

            Value::StructType(
                ty
            ) => {
                ensure_positional_args(
                    names
                )?;

                self.make_struct_value(
                    ty,
                    args,
                )
            }

            other => {
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "value is not callable: {}",
                            other.type_name()
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn make_enum_value(
        &mut self,
        constructor: EnumConstructor,
        args: Vec<Value>,
    ) -> Result<Value> {
        let expected =
            constructor.arity();

        if args.len() != expected {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{} expects {} arguments, got {}",
                        constructor,
                        expected,
                        args.len(),
                    ),
                    None,
                )
            );
        }

        Ok(
            Value::EnumValue(
                Rc::new(
                    EnumValue::new(
                        constructor.enum_def().name(),
                        constructor.variant(),
                        args,
                    )
                )
            )
        )
    }

    fn make_struct_value(
        &mut self,
        ty: StructTypeRef,
        args: Vec<Value>,
    ) -> Result<Value> {
        let expected =
            ty.fields().len();

        if args.len() != expected {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{} expects {} arguments, got {}",
                        ty.name(),
                        expected,
                        args.len(),
                    ),
                    None,
                )
            );
        }

        let value =
            StructValue::new(
                ty,
                args,
            )
            .map_err(|message| {
                Error::new(
                    ErrorKind::Type,
                    message,
                    None,
                )
            })?;

        Ok(
            Value::Struct(
                Rc::new(value)
            )
        )
    }

    fn eval_pipeline_expr(
        &self,
        expr: &PipelineExpr,
        input: Value,
        captures: &[CellRef],
    ) -> Result<Value> {
        match expr {
            PipelineExpr::Input =>
                Ok(input),

            PipelineExpr::Int(value) =>
                Ok(
                    Value::Int(*value)
                ),

            PipelineExpr::Float(value) =>
                Ok(
                    Value::Float(*value)
                ),

            PipelineExpr::Bool(value) =>
                Ok(
                    Value::Bool(*value)
                ),

            PipelineExpr::Str(value) =>
                Ok(
                    Value::Str(
                        Rc::new(
                            value.clone()
                        )
                    )
                ),

            PipelineExpr::Capture(index) => {
                let cell =
                    captures
                        .get(*index as usize)
                        .cloned()
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                format!(
                                    "pipeline capture slot out of bounds: {}",
                                    index
                                ),
                                None,
                            )
                        })?;

                let value =
                    cell.borrow().clone();

                Ok(value)
            }

            PipelineExpr::Add(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Add,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Sub(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Sub,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Mul(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Mul,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Div(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Div,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Mod(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Mod,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Pow(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Pow,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Eq(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Eq,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Neq(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Neq,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Lt(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Lt,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Leq(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Leq,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Gt(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Gt,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Geq(
                left,
                right,
            ) => {
                self.eval_pipeline_binary(
                    BinOp::Geq,
                    left,
                    right,
                    input,
                    captures,
                )
            }

            PipelineExpr::Neg(
                expr
            ) => {
                let value =
                    self.eval_pipeline_expr(
                        expr,
                        input,
                        captures,
                    )?;

                value
                    .negate()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Runtime,
                                message,
                                None,
                            )
                        }
                    )
            }

            PipelineExpr::Not(
                expr
            ) => {
                let value =
                    self.eval_pipeline_expr(
                        expr,
                        input,
                        captures,
                    )?;

                match value {
                    Value::Bool(value) =>
                        Ok(
                            Value::Bool(!value)
                        ),

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
        }
    }

    fn eval_pipeline_binary(
        &self,
        op: BinOp,
        left: &PipelineExpr,
        right: &PipelineExpr,
        input: Value,
        captures: &[CellRef],
    ) -> Result<Value> {
        let lhs =
            self.eval_pipeline_expr(
                left,
                input.clone(),
                captures,
            )?;

        let rhs =
            self.eval_pipeline_expr(
                right,
                input,
                captures,
            )?;

        apply_binop(
            op,
            lhs,
            rhs,
        )
        .map_err(
            |message| {
                Error::new(
                    ErrorKind::Runtime,
                    message,
                    None,
                )
            }
        )
    }

    fn eval_int_pipeline_expr(
        &self,
        expr: &IntPipelineExpr,
        input: i64,
        captures: &[CellRef],
    ) -> Result<i64> {
        match expr {
            IntPipelineExpr::Input =>
                Ok(input),

            IntPipelineExpr::Const(value) =>
                Ok(*value),

            IntPipelineExpr::Capture(index) => {
                let cell =
                    captures
                        .get(
                            *index as usize
                        )
                        .cloned()
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Runtime,
                                format!(
                                    "pipeline capture slot out of bounds: {}",
                                    index
                                ),
                                None,
                            )
                        })?;

                let value =
                    cell.borrow().clone();

                match value {
                    Value::Int(value) =>
                        Ok(value),

                    other =>
                        Err(
                            Error::new(
                                ErrorKind::Type,
                                format!(
                                    "integer pipeline capture must be Int, got {}",
                                    other.type_name()
                                ),
                                None,
                            )
                        ),
                }
            }

            IntPipelineExpr::Add(
                left,
                right,
            ) => {
                let lhs =
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?;

                let rhs =
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?;

                lhs.checked_add(
                    rhs
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "integer addition overflow",
                        None,
                    )
                })
            }

            IntPipelineExpr::Sub(
                left,
                right,
            ) => {
                let lhs =
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?;

                let rhs =
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?;

                lhs.checked_sub(
                    rhs
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "integer subtraction overflow",
                        None,
                    )
                })
            }

            IntPipelineExpr::Mul(
                left,
                right,
            ) => {
                let lhs =
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?;

                let rhs =
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?;

                lhs.checked_mul(
                    rhs
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "integer multiplication overflow",
                        None,
                    )
                })
            }

            IntPipelineExpr::Div(
                left,
                right,
            ) => {
                let lhs =
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?;

                let rhs =
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?;

                lhs.checked_div(
                    rhs
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "integer division failed",
                        None,
                    )
                })
            }

            IntPipelineExpr::Mod(
                left,
                right,
            ) => {
                let lhs =
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?;

                let rhs =
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?;

                lhs.checked_rem(
                    rhs
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "integer remainder failed",
                        None,
                    )
                })
            }

            IntPipelineExpr::Neg(
                expr
            ) => {
                self.eval_int_pipeline_expr(
                    expr,
                    input,
                    captures,
                )?
                .checked_neg()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "integer negation overflow",
                        None,
                    )
                })
            }
        }
    }

    fn eval_int_pipeline_predicate(
        &self,
        predicate: &IntPipelinePredicate,
        input: i64,
        captures: &[CellRef],
    ) -> Result<bool> {
        match predicate {
            IntPipelinePredicate::Eq(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    ==
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),

            IntPipelinePredicate::Neq(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    !=
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),

            IntPipelinePredicate::Lt(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    <
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),

            IntPipelinePredicate::Leq(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    <=
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),

            IntPipelinePredicate::Gt(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    >
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),

            IntPipelinePredicate::Geq(
                left,
                right,
            ) =>
                Ok(
                    self.eval_int_pipeline_expr(
                        left,
                        input,
                        captures,
                    )?
                    >=
                    self.eval_int_pipeline_expr(
                        right,
                        input,
                        captures,
                    )?
                ),
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

    fn try_value(
        &mut self,
        value: Value,
        target_depth: usize,
    ) -> Result<Option<ExecutionResult>> {
        let Value::EnumValue(
            enum_value
        ) = value.clone()
        else {
            return Err(
                Error::new(
                    ErrorKind::Type,
                    format!(
                        "the '?' operator requires Result or Option, got {}",
                        value.type_name()
                    ),
                    None,
                )
            );
        };

        match (
            enum_value.enum_name(),
            enum_value.variant(),
        ) {
            ("Option", "Some") => {
                if enum_value.fields().len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "Option::Some must contain one value",
                            None,
                        )
                    );
                }

                self.push(
                    enum_value.field(0)
                        .expect("checked above")
                );

                Ok(None)
            }

            ("Result", "Ok") => {
                if enum_value.fields().len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "Result::Ok must contain one value",
                            None,
                        )
                    );
                }

                self.push(
                    enum_value.field(0)
                        .expect("checked above")
                );

                Ok(None)
            }

            ("Option", "None") => {
                if !enum_value.fields().is_empty() {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "Option::None must not contain values",
                            None,
                        )
                    );
                }

                self.return_from_current_frame(
                    value,
                    target_depth,
                )
            }

            ("Result", "Err") => {
                if enum_value.fields().len() != 1 {
                    return Err(
                        Error::new(
                            ErrorKind::Runtime,
                            "Result::Err must contain one value",
                            None,
                        )
                    );
                }

                self.return_from_current_frame(
                    value,
                    target_depth,
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "'?' is only supported for Option or Result, got {}.{}",
                            enum_value.enum_name(),
                            enum_value.variant(),
                        ),
                        None,
                    )
                )
            }
        }
    }

    #[inline(always)]
    fn advance_range(
        &mut self,
        range_index: usize,
    ) -> Result<()> {
        let range =
            self.current_frame()
                .closure
                .function
                .chunk
                .range_loops
                .get(range_index)
                .copied()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "range loop index out of bounds",
                        None,
                    )
                })?;

        let frame =
            self.current_frame_mut();

        let cursor =
            frame
                .range_cursors
                .get_mut(range_index)
                .and_then(Option::as_mut)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Runtime,
                        "range cursor is not initialized",
                        None,
                    )
                })?;

        if cursor.current >= cursor.end {
            frame.ip =
                range.exit_ip as usize;

            return Ok(());
        }

        let current =
            cursor.current;

        cursor.current =
            current.checked_add(1)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "range increment overflow",
                        None,
                    )
                })?;

        let value =
            Value::Int(current);

        let value_slot =
            range.value_slot as usize;

        /*
        * Local storage invariant.
        */
        if frame.locals.len()
            <= value_slot
        {
            frame.locals.resize(
                value_slot + 1,
                Value::Unit,
            );
        }

        frame.locals[value_slot] =
            value.clone();

        /*
        * Capture storage invariant.
        */
        if let Some(cells) =
            frame.cells.as_mut()
        {
            if cells.len() <= value_slot {
                cells.resize(
                    value_slot + 1,
                    None,
                );
            }

            if let Some(cell) =
                cells[value_slot].as_ref()
            {
                *cell.borrow_mut() =
                    value;
            }
        }

        Ok(())
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

            OpCode::MatMul =>
                Some(BinOp::MatMul),

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
            apply_binop(
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
                        "local slot out of bounds: {}",
                        slot
                    ),
                    None,
                )
            );
        }

        if frame.cells.is_none() {
            let local_count =
                frame.locals.len();

            frame.cells =
                Some(
                    vec![
                        None;
                        local_count
                    ]
                );
        }

        let cells =
            frame.cells
                .as_mut()
                .expect(
                    "cells must be initialized"
                );

        if cells.len() <= slot {
            cells.resize(
                slot + 1,
                None,
            );
        }

        if let Some(cell) =
            cells[slot].clone()
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

        cells[slot] =
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

    fn load_module(
        &mut self,
        path: &ModulePath,
    ) -> Result<ModuleRef> {
        let importing_file =
            self.current_frame()
                .source_path
                .clone();

        /*
        * 1. User module
        */
        if let Some(canonical) =
            self.module_loader.try_resolve(
                path,
                importing_file.as_deref(),
            )?
        {
            return self.load_file_module(
                canonical,
                path,
            );
        }

        /*
        * 2. Standard library
        *
        * Only single-component names are
        * stdlib modules for now.
        */
        if path.parts().len() == 1 {
            let name =
                &path.parts()[0];

            if let Some(module) =
                self.stdlib_modules
                    .get(name)
            {
                return Ok(
                    module.clone()
                );
            }

            if let Some(module) =
                crate::stdlib::load_module(
                    name
                )
            {
                self.stdlib_modules.insert(
                    name.clone(),
                    module.clone(),
                );

                return Ok(module);
            }
        }

        Err(
            Error::new(
                ErrorKind::Import,
                format!(
                    "module '{}' not found",
                    path
                ),
                None,
            )
        )
    }

    fn load_module_namespace(
        &mut self,
        path: &ModulePath,
    ) -> Result<ModuleRef> {
        let parts =
            path.parts();

        if parts.is_empty() {
            return Err(
                Error::new(
                    ErrorKind::Import,
                    "empty module path",
                    None,
                )
            );
        }

        /*
        * A single-component import does not need
        * a synthetic namespace.
        *
        * import math
        *
        * becomes:
        *
        * math -> actual math ModuleRef
        */
        if parts.len() == 1 {
            return self.load_module(path);
        }

        /*
        * Load the actual leaf module first.
        *
        * a.b.c
        *       ↓
        * actual c ModuleRef
        */
        let leaf =
            self.load_module(path)?;

        /*
        * Ensure every synthetic namespace exists:
        *
        * a
        * a.b
        */
        for depth in
            0..parts.len() - 1
        {
            let prefix =
                ModulePath::new(
                    parts[..=depth]
                        .to_vec()
                );

            if !self.module_namespaces
                .contains_key(&prefix)
            {
                let module =
                    Rc::new(
                        RefCell::new(
                            Module::new(
                                prefix.name()
                            )
                        )
                    );

                self.module_namespaces.insert(
                    prefix,
                    module,
                );
            }
        }

        /*
        * Link namespaces:
        *
        * a
        * └── b
        *
        * a.b.c
        *      ↓
        * b exports c
        */
        for depth in
            0..parts.len() - 2
        {
            let parent_path =
                ModulePath::new(
                    parts[..=depth]
                        .to_vec()
                );

            let child_path =
                ModulePath::new(
                    parts[..=depth + 1]
                        .to_vec()
                );

            let parent =
                self.module_namespaces
                    .get(&parent_path)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Import,
                            format!(
                                "internal error: namespace '{}' was not created",
                                parent_path
                            ),
                            None,
                        )
                    })?;

            let child =
                self.module_namespaces
                    .get(&child_path)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Import,
                            format!(
                                "internal error: namespace '{}' was not created",
                                child_path
                            ),
                            None,
                        )
                    })?;

            parent
                .borrow_mut()
                .set_exported(
                    parts[depth + 1].clone(),
                    Value::Module(child),
                );
        }

        /*
        * Attach the actual leaf module.
        *
        * a.b
        *   └── c -> actual c ModuleRef
        */
        let parent_depth =
            parts.len() - 2;

        let parent_path =
            ModulePath::new(
                parts[..=parent_depth]
                    .to_vec()
            );

        let parent =
            self.module_namespaces
                .get(&parent_path)
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Import,
                        format!(
                            "internal error: namespace '{}' was not created",
                            parent_path
                        ),
                        None,
                    )
                })?;

        parent
            .borrow_mut()
            .set_exported(
                parts[parts.len() - 1].clone(),
                Value::Module(leaf),
            );

        /*
        * Return the root namespace.
        *
        * import a.b.c
        *
        * binds:
        *
        * a -> Module("a")
        */
        let root_path =
            ModulePath::new(
                vec![
                    parts[0].clone()
                ]
            );

        self.module_namespaces
            .get(&root_path)
            .cloned()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Import,
                    format!(
                        "internal error: root namespace '{}' was not created",
                        root_path
                    ),
                    None,
                )
            })
    }

    fn load_file_module(
        &mut self,
        canonical: PathBuf,
        module_path: &ModulePath,
    ) -> Result<ModuleRef> {
        if self.loading_modules
            .contains(&canonical)
        {
            let mut chain =
                self.loading_modules
                    .iter()
                    .map(|path| {
                        path.display()
                            .to_string()
                    })
                    .collect::<Vec<_>>();

            chain.push(
                canonical
                    .display()
                    .to_string()
            );

            return Err(
                Error::new(
                    ErrorKind::Import,
                    format!(
                        "cyclic module import: {}",
                        chain.join(" -> ")
                    ),
                    None,
                )
            );
        }

        if let Some(module) =
            self.modules.get(&canonical)
        {
            return Ok(
                module.clone()
            );
        }

        let module =
            Rc::new(
                RefCell::new(
                    Module::new(
                        module_path.name()
                    )
                )
            );

        self.modules.insert(
            canonical.clone(),
            module.clone(),
        );

        self.loading_modules.push(
            canonical.clone()
        );

        let chunk =
            match self.module_loader
                .load_chunk(&canonical)
            {
                Ok(chunk) =>
                    chunk,

                Err(error) => {
                    self.loading_modules.pop();
                    self.modules.remove(
                        &canonical
                    );

                    return Err(error);
                }
            };

        let result =
            self.execute_module(
                chunk,
                module.clone(),
                canonical.clone(),
            );

        self.loading_modules.pop();

        match result {
            Ok(_) =>
                Ok(module),

            Err(error) => {
                self.modules.remove(
                    &canonical
                );

                Err(error)
            }
        }
    }

    // ============================
    //   Iterator
    // ============================
    fn iterator_next(
        &mut self,
        iterator: IteratorRef,
    ) -> Result<IterResult> {
        let kind =
            {
                let state =
                    iterator.borrow();

                match &*state {
                    IteratorObj::List { .. } =>
                        0,

                    IteratorObj::Str { .. } =>
                        1,

                    IteratorObj::Vector { .. } =>
                        2,

                    IteratorObj::Range { .. } =>
                        3,

                    IteratorObj::Map { .. } =>
                        4,

                    IteratorObj::Filter { .. } =>
                        5,

                    IteratorObj::Enumerate { .. } =>
                        6,

                    IteratorObj::Zip { .. } =>
                        7,

                    IteratorObj::Take { .. } =>
                        8,

                    IteratorObj::Skip { .. } =>
                        9,

                    IteratorObj::Series { .. } =>
                        10,

                    IteratorObj::DataFrame { .. } =>
                        11,
                }
            };

        match kind {
            0 | 1 | 2 | 3 => {
                self.iterator_next_base(
                    &iterator
                )
            }

            4 => {
                let (
                    source,
                    function,
                ) = {
                    let state =
                        iterator.borrow();

                    let IteratorObj::Map {
                        source,
                        function,
                    } = &*state
                    else {
                        unreachable!();
                    };

                    (
                        source.clone(),
                        function.clone(),
                    )
                };

                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        let result =
                            self.call_closure_sync1(
                                function,
                                value,
                            )?;

                        Ok(
                            IterResult::Item(
                                result
                            )
                        )
                    }
                }
            }

            5 => {
                let (
                    source,
                    predicate,
                ) = {
                    let state =
                        iterator.borrow();

                    let IteratorObj::Filter {
                        source,
                        predicate,
                    } = &*state
                    else {
                        unreachable!();
                    };

                    (
                        source.clone(),
                        predicate.clone(),
                    )
                };

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
                                self.call_closure_sync1(
                                    predicate.clone(),
                                    value.clone(),
                                )?;

                            match result {
                                Value::Bool(true) =>
                                    return Ok(
                                        IterResult::Item(
                                            value
                                        )
                                    ),

                                Value::Bool(false) => {}

                                other =>
                                    return Err(
                                        Error::new(
                                            ErrorKind::Type,
                                            format!(
                                                "filter predicate must return Bool, got {}",
                                                other.type_name()
                                            ),
                                            None,
                                        )
                                    ),
                            }
                        }
                    }
                }
            }

            6 => {
                let source =
                    {
                        let state =
                            iterator.borrow();

                        let IteratorObj::Enumerate {
                            source,
                            ..
                        } = &*state
                        else {
                            unreachable!();
                        };

                        source.clone()
                    };

                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        let current =
                            {
                                let mut state =
                                    iterator.borrow_mut();

                                let IteratorObj::Enumerate {
                                    index,
                                    ..
                                } = &mut *state
                                else {
                                    unreachable!();
                                };

                                let current =
                                    *index;

                                *index += 1;

                                current
                            };

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
                    }
                }
            }

            7 => {
                let (
                    left,
                    right,
                ) = {
                    let state =
                        iterator.borrow();

                    let IteratorObj::Zip {
                        left,
                        right,
                    } = &*state
                    else {
                        unreachable!();
                    };

                    (
                        left.clone(),
                        right.clone(),
                    )
                };

                let left =
                    self.iterator_next(
                        left
                    )?;

                let right =
                    self.iterator_next(
                        right
                    )?;

                match (
                    left,
                    right,
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

            8 => {
                let source =
                    {
                        let state =
                            iterator.borrow();

                        let IteratorObj::Take {
                            source,
                            remaining,
                        } = &*state
                        else {
                            unreachable!();
                        };

                        if *remaining == 0 {
                            return Ok(
                                IterResult::End
                            );
                        }

                        source.clone()
                    };

                match self.iterator_next(
                    source
                )? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        let mut state =
                            iterator.borrow_mut();

                        let IteratorObj::Take {
                            remaining,
                            ..
                        } = &mut *state
                        else {
                            unreachable!();
                        };

                        *remaining -= 1;

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }
                }
            }

            9 => {
                let source =
                    {
                        let state =
                            iterator.borrow();

                        let IteratorObj::Skip {
                            source,
                            remaining: _,
                        } = &*state
                        else {
                            unreachable!();
                        };

                        source.clone()
                    };

                loop {
                    let remaining =
                        {
                            let state =
                                iterator.borrow();

                            let IteratorObj::Skip {
                                remaining,
                                ..
                            } = &*state
                            else {
                                unreachable!();
                            };

                            *remaining
                        };

                    if remaining == 0 {
                        break;
                    }

                    match self.iterator_next(
                        source.clone()
                    )? {
                        IterResult::End =>
                            return Ok(
                                IterResult::End
                            ),

                        IterResult::Item(_) => {
                            let mut state =
                                iterator.borrow_mut();

                            let IteratorObj::Skip {
                                remaining,
                                ..
                            } = &mut *state
                            else {
                                unreachable!();
                            };

                            *remaining -= 1;
                        }
                    }
                }

                self.iterator_next(
                    source
                )
            }

            10 | 11 => {
                self.iterator_next_base(
                    &iterator
                )
            }

            _ =>
                unreachable!(),
        }
    }

    fn iterator_next_base(
        &mut self,
        iterator: &IteratorRef,
    ) -> Result<IterResult> {
        let kind =
            {
                let state =
                    iterator.borrow();

                match &*state {
                    IteratorObj::List { .. } =>
                        0,

                    IteratorObj::Str { .. } =>
                        1,

                    IteratorObj::Vector { .. } =>
                        2,

                    IteratorObj::Series { .. } =>
                        3,

                    IteratorObj::DataFrame { .. } =>
                        4,

                    IteratorObj::Range { .. } =>
                        5,

                    _ =>
                        return Err(
                            Error::new(
                                ErrorKind::Runtime,
                                "iterator is not a base iterator",
                                None,
                            )
                        ),
                }
            };

        match kind {
            /*
            * --------------------------------------------------
            * List
            * --------------------------------------------------
            */
            0 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::List {
                    data,
                    index,
                } = &mut *state
                else {
                    unreachable!();
                };

                match data.get(*index) {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * --------------------------------------------------
            * String
            * --------------------------------------------------
            */
            1 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::Str {
                    data,
                    byte_index,
                } = &mut *state
                else {
                    unreachable!();
                };

                let slice =
                    &data[*byte_index..];

                let Some(ch) =
                    slice.chars().next()
                else {
                    return Ok(
                        IterResult::End
                    );
                };

                *byte_index +=
                    ch.len_utf8();

                Ok(
                    IterResult::Item(
                        Value::Str(
                            Rc::new(
                                ch.to_string()
                            )
                        )
                    )
                )
            }

            /*
            * --------------------------------------------------
            * Vector
            * --------------------------------------------------
            */
            2 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::Vector {
                    data,
                    index,
                } = &mut *state
                else {
                    unreachable!();
                };

                let x = match data.borrow().get(*index) {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                Value::Float(
                                    value
                                )
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }; x
            }

            /*
            * --------------------------------------------------
            * Series
            * --------------------------------------------------
            */
            3 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::Series {
                    data,
                    index,
                } = &mut *state
                else {
                    unreachable!();
                };

                match data.get(*index) {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * --------------------------------------------------
            * DataFrame
            *
            * A DataFrame iterates over rows.
            *
            * row(i) -> Dict
            * --------------------------------------------------
            */
            4 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::DataFrame {
                    data,
                    index,
                } = &mut *state
                else {
                    unreachable!();
                };

                match data.row(*index) {
                    Some(row) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                Value::Dict(row)
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * --------------------------------------------------
            * Range
            * --------------------------------------------------
            */
            5 => {
                let mut state =
                    iterator.borrow_mut();

                let IteratorObj::Range {
                    current,
                    end,
                } = &mut *state
                else {
                    unreachable!();
                };

                if *current >= *end {
                    return Ok(
                        IterResult::End
                    );
                }

                let value =
                    *current;

                *current += 1;

                Ok(
                    IterResult::Item(
                        Value::Int(
                            value
                        )
                    )
                )
            }

            _ =>
                unreachable!(),
        }
    }

    fn collect_iterator(
        &mut self,
        iterator: IteratorRef,
    ) -> Result<Value> {
        let list =
            List::with_capacity(0);

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
                        self.call_closure_sync2(
                            function.clone(),
                            accumulator,
                            value,
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
                        self.call_closure_sync2(
                            function.clone(),
                            accumulator,
                            value,
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
                        self.call_closure_sync1(
                            predicate.clone(),
                            value,
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
                        self.call_closure_sync1(
                            predicate.clone(),
                            value,
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
                        apply_binop(
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
                        apply_binop(
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

    fn make_iterator(
        &self,
        value: Value,
    ) -> Result<IteratorRef> {
        IteratorObj::from_value(
            value
        )
        .map_err(|message| {
            Error::new(
                ErrorKind::Type,
                message,
                None,
            )
        })
    }

    /// Handles any type of method dispatches
    /// List, Struct, Object, ... etc
    fn invoke_method(
        &mut self,
        receiver: Value,
        name: &str,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        match receiver {
            Value::Module(module) =>
                self.invoke_module_member(
                    module,
                    name,
                    args,
                    names,
                ),

            Value::Object(object) =>
                self.invoke_object_method(
                    object,
                    name,
                    args,
                    names,
                ),

            Value::List(list) =>
                self.invoke_list_method(
                    list,
                    name,
                    args,
                ),

            Value::Str(string) =>
                self.invoke_string_method(
                    string,
                    name,
                    args,
                ),

            Value::Series(series) =>
                self.invoke_series_method(
                    series,
                    name,
                    args,
                ),

            Value::DataFrame(df) =>
                self.invoke_dataframe_method(
                    df,
                    name,
                    args,
                ),

            Value::Iterator(iterator) =>
                self.invoke_iterator_method(
                    iterator,
                    name,
                    args,
                ),

            Value::Range(
                start,
                end,
                inclusive,
            ) =>
                self.invoke_range_method(
                    start,
                    end,
                    inclusive,
                    name,
                    args,
                ),

            Value::Enum(enum_def) =>
                self.invoke_enum_constructor(
                    enum_def,
                    name,
                    args,
                ),

            Value::Path(path) =>
                self.invoke_path_method(
                    path,
                    name,
                    args,
                ),

            other =>
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "method '{}' is not supported for this value ({})",
                            name,
                            other.type_name(),
                        ),
                        None,
                    )
                ),
        }
    }

    fn invoke_module_member(
        &mut self,
        module: ModuleRef,
        name: &str,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        let value =
            module
                .borrow()
                .get_field(name)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "module '{}' has no field '{}'",
                            module.borrow().name(),
                            name,
                        ),
                        None,
                    )
                })?;

        match value {
            Value::Builtin(function) => {
                ensure_positional_args(
                    names
                )?;

                function(args)
                    .map_err(|message| {
                        Error::new(
                            ErrorKind::Runtime,
                            message,
                            None,
                        )
                    })
            }

            Value::Closure(closure) => {
                self.call_closure_sync_named(
                    closure,
                    args,
                    names,
                )
            }

            Value::Class(class) => {
                self.call_class_value(
                    class,
                    args,
                    names,
                )
            }

            other => {
                Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "module field '{}' is not callable (got {})",
                            name,
                            other.type_name(),
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
                        List::new(
                            values
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
        list: List,
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

    fn invoke_series_method(
        &mut self,
        series: SeriesRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "is_null" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Series(
                        Rc::new(
                            series.is_null()
                        )
                    )
                )
            }

            "is_not_null" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Series(
                        Rc::new(
                            series.is_not_null()
                        )
                    )
                )
            }

            "dropna" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Series(
                        Rc::new(
                            series.dropna()
                        )
                    )
                )
            }

            "unique" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let value =
                    series
                        .unique()
                        .map_err(
                            |message| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                )
                            }
                        )?;

                Ok(
                    Value::Series(
                        Rc::new(value)
                    )
                )
            }

            "mean" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .mean()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "sum" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .sum()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "min" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .min()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "max" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .max()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "std" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .std()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "median" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                series
                    .median()
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Type,
                                message,
                                None,
                            )
                        }
                    )
            }

            "quantile" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let q =
                    match args[0] {
                        Value::Float(q) =>
                            q,

                        Value::Int(q) =>
                            q as f64,

                        ref other =>
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    format!(
                                        "quantile() expects Float or Int, got {}",
                                        other.type_name()
                                    ),
                                    None,
                                )
                            ),
                    };

                series
                    .quantile(q)
                    .map_err(
                        |message| {
                            Error::new(
                                ErrorKind::Value,
                                message,
                                None,
                            )
                        }
                    )
            }

            "with_name" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    new_name
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "with_name() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                Ok(
                    Value::Series(
                        Rc::new(
                            series.with_name(
                                new_name.as_str()
                            )
                        )
                    )
                )
            }

            "to_matrix" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let matrix =
                    series
                        .to_matrix()
                        .map_err(
                            |message| {
                                Error::new(
                                    ErrorKind::Type,
                                    message,
                                    None,
                                )
                            }
                        )?;

                Ok(
                    Value::Matrix(
                        Rc::new(
                            RefCell::new(matrix)
                        )
                    )
                )
            }

            "iter" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let iterator =
                    self.make_iterator(
                        Value::Series(series)
                    )?;

                Ok(
                    Value::Iterator(
                        iterator
                    )
                )
            }

            _ => Err(
                Error::new(
                    ErrorKind::Name,
                    format!(
                        "Series has no method '{}'",
                        name
                    ),
                    None,
                )
            ),
        }
    }

    fn invoke_dataframe_method(
        &mut self,
        df: DataFrameRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            "column" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let Value::Str(
                    name
                ) = &args[0]
                else {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "column() expects Str, got {}",
                                args[0].type_name()
                            ),
                            None,
                        )
                    );
                };

                let column =
                    df.column(
                        name.as_str()
                    )
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Name,
                            format!(
                                "unknown DataFrame column '{}'",
                                name
                            ),
                            None,
                        )
                    })?;

                Ok(
                    Value::Series(
                        column
                    )
                )
            }

            "row" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let index =
                    self.expect_usize_index(
                        name,
                        &args[0],
                    )?;

                let row =
                    df.row(index)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::Index,
                                format!(
                                    "DataFrame row index out of bounds: {}",
                                    index
                                ),
                                None,
                            )
                        })?;

                Ok(
                    Value::Dict(row)
                )
            }

            "take_rows" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let indices =
                    self.expect_usize_indices(
                        name,
                        &args[0],
                    )?;

                let result =
                    df.take_rows(
                        &indices
                    )
                    .map_err(|message| {
                        Error::new(
                            ErrorKind::Runtime,
                            message,
                            None,
                        )
                    })?;

                Ok(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                )
            }

            "head" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let n =
                    self.expect_usize_index(
                        name,
                        &args[0],
                    )?;

                let result =
                    df.head(n)
                        .map_err(
                            |message| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                )
                            }
                        )?;

                Ok(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                )
            }

            "describe" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let result =
                    df.describe()
                        .map_err(
                            |message| {
                                Error::new(
                                    ErrorKind::Runtime,
                                    message,
                                    None,
                                )
                            }
                        )?;

                Ok(
                    Value::DataFrame(
                        Rc::new(result)
                    )
                )
            }

            "to_matrix" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let result =
                    df.to_matrix()
                        .map_err(
                            |message| {
                                Error::new(
                                    ErrorKind::Type,
                                    message,
                                    None,
                                )
                            }
                        )?;

                Ok(
                    Value::Matrix(
                        Rc::new(
                            RefCell::new(result)
                        )
                    )
                )
            }

            "iter" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                let iterator =
                    self.make_iterator(
                        Value::DataFrame(df)
                    )?;

                Ok(
                    Value::Iterator(
                        iterator
                    )
                )
            }

            _ => Err(
                Error::new(
                    ErrorKind::Name,
                    format!(
                        "DataFrame has no method '{}'",
                        name
                    ),
                    None,
                )
            ),
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

    fn invoke_object_method(
        &mut self,
        object: ObjectRef,
        name: &str,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value> {
        if name == "new" {
            return Err(
                Error::new(
                    ErrorKind::Name,
                    "class constructor 'new' can only be called through the class",
                    None,
                )
            );
        }

        let class = {
            let object_ref =
                object.borrow();

            object_ref.class()
        };

        let method =
            class
                .method(name)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "class '{}' has no method '{}'",
                            class.name(),
                            name,
                        ),
                        None,
                    )
                })?;

        let mut call_args =
            Vec::with_capacity(
                args.len() + 1
            );

        let mut call_names =
            Vec::with_capacity(
                names.len() + 1
            );

        // self is always positional parameter 0.
        call_args.push(
            Value::Object(
                object
            )
        );

        call_names.push(None);

        call_args.extend(
            args
        );

        call_names.extend(
            names.iter().cloned()
        );

        self.call_closure_sync_named(
            method,
            call_args,
            &call_names,
        )
    }

    fn invoke_path_method(
        &mut self,
        path: crate::runtime::PathRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        match name {
            /*
            * ----------------------------------------------------
            * name()
            * ----------------------------------------------------
            *
            * Returns the final component of the path.
            *
            *     path("foo/bar.txt").name()
            *         -> Some("bar.txt")
            */
            "name" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                match path.name() {
                    Some(value) =>
                        Ok(
                            option_some(
                                Value::Str(
                                    Rc::new(value)
                                )
                            )
                        ),

                    None =>
                        Ok(
                            option_none()
                        ),
                }
            }

            /*
            * ----------------------------------------------------
            * extension()
            * ----------------------------------------------------
            */
            "extension" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                match path.extension() {
                    Some(value) =>
                        Ok(
                            option_some(
                                Value::Str(
                                    Rc::new(value)
                                )
                            )
                        ),

                    None =>
                        Ok(
                            option_none()
                        ),
                }
            }

            /*
            * ----------------------------------------------------
            * stem()
            * ----------------------------------------------------
            */
            "stem" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                match path.stem() {
                    Some(value) =>
                        Ok(
                            option_some(
                                Value::Str(
                                    Rc::new(value)
                                )
                            )
                        ),

                    None =>
                        Ok(
                            option_none()
                        ),
                }
            }

            /*
            * ----------------------------------------------------
            * parent()
            * ----------------------------------------------------
            */
            "parent" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                match path.parent() {
                    Some(parent) =>
                        Ok(
                            option_some(
                                Value::Path(
                                    Rc::new(parent)
                                )
                            )
                        ),

                    None =>
                        Ok(
                            option_none()
                        ),
                }
            }

            /*
            * ----------------------------------------------------
            * join(path)
            * ----------------------------------------------------
            *
            * Accept both Str and Path:
            *
            *     p.join("file.txt")
            *     p.join(path("file.txt"))
            */
            "join" => {
                self.expect_arity(
                    name,
                    &args,
                    1,
                )?;

                let child =
                    match &args[0] {
                        Value::Str(value) => {
                            std::path::PathBuf::from(
                                value.as_ref()
                            )
                        }

                        Value::Path(value) => {
                            value.to_path_buf()
                        }

                        other => {
                            return Err(
                                Error::new(
                                    ErrorKind::Type,
                                    format!(
                                        "join() expects Str or Path, got {}",
                                        other.type_name()
                                    ),
                                    None,
                                )
                            );
                        }
                    };

                Ok(
                    Value::Path(
                        Rc::new(
                            path.join(&child)
                        )
                    )
                )
            }

            /*
            * ----------------------------------------------------
            * exists()
            * ----------------------------------------------------
            */
            "exists" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Bool(
                        path.exists()
                    )
                )
            }

            /*
            * ----------------------------------------------------
            * is_file()
            * ----------------------------------------------------
            */
            "is_file" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Bool(
                        path.is_file()
                    )
                )
            }

            /*
            * ----------------------------------------------------
            * is_dir()
            * ----------------------------------------------------
            */
            "is_dir" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Bool(
                        path.is_dir()
                    )
                )
            }

            /*
            * ----------------------------------------------------
            * string()
            * ----------------------------------------------------
            *
            * Explicit conversion method.
            *
            * `str(path)` already exists, but `string()` makes
            * Path's public API self-contained.
            */
            "string" => {
                self.expect_arity(
                    name,
                    &args,
                    0,
                )?;

                Ok(
                    Value::Str(
                        Rc::new(
                            path.to_string_lossy()
                        )
                    )
                )
            }

            _ => {
                Err(
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "Path has no method '{}'",
                            name
                        ),
                        None,
                    )
                )
            }
        }
    }

    fn invoke_enum_constructor(
        &mut self,
        enum_def: EnumRef,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        let variant =
            enum_def
                .variant(name)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Name,
                        format!(
                            "enum '{}' has no variant '{}'",
                            enum_def.name(),
                            name,
                        ),
                        None,
                    )
                })?;

        if args.len() != variant.arity() {
            return Err(
                Error::new(
                    ErrorKind::Arity,
                    format!(
                        "{}.{} expects {} arguments, got {}",
                        enum_def.name(),
                        name,
                        variant.arity(),
                        args.len(),
                    ),
                    None,
                )
            );
        }

        Ok(
            Value::EnumValue(
                Rc::new(
                    EnumValue::new(
                        enum_def.name(),
                        name,
                        args,
                    )
                )
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

    fn expect_usize_index(
        &self,
        name: &str,
        value: &Value,
    ) -> Result<usize> {
        match value {
            Value::Int(index)
                if *index >= 0 =>
            {
                Ok(*index as usize)
            }

            Value::Int(_) => {
                Err(
                    Error::new(
                        ErrorKind::Index,
                        format!(
                            "{}() index must be non-negative",
                            name
                        ),
                        None,
                    )
                )
            }

            other => {
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
    }

    fn expect_usize_indices(
        &self,
        name: &str,
        value: &Value,
    ) -> Result<Vec<usize>> {
        let Value::List(list) =
            value
        else {
            return Err(
                Error::new(
                    ErrorKind::Type,
                    format!(
                        "{}() expects List[Int]",
                        name
                    ),
                    None,
                )
            );
        };

        let values =
            list.as_vec();

        let mut result =
            Vec::with_capacity(
                values.len()
            );

        for value in values.iter() {
            match value {
                Value::Int(index)
                    if *index >= 0 =>
                {
                    result.push(
                        *index as usize
                    );
                }

                Value::Int(_) => {
                    return Err(
                        Error::new(
                            ErrorKind::Index,
                            format!(
                                "{}() index must be non-negative",
                                name
                            ),
                            None,
                        )
                    );
                }

                other => {
                    return Err(
                        Error::new(
                            ErrorKind::Type,
                            format!(
                                "{}() expects List[Int], found {}",
                                name,
                                other.type_name()
                            ),
                            None,
                        )
                    );
                }
            }
        }

        Ok(result)
    }

}

#[inline]
fn ensure_positional_args(
    names: &[Option<String>],
) -> Result<()> {
    if names.iter()
        .any(Option::is_some)
    {
        return Err(
            Error::new(
                ErrorKind::Runtime,
                "named arguments are not supported for this method",
                None,
            )
        );
    }

    Ok(())
}