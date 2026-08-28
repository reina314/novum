use std::{
    cell::RefCell,
    rc::Rc,
    path::PathBuf,
};

use super::{
    Value,
    ModuleRef,
};
use crate::vm::{
    Chunk,
};

pub type CellRef = Rc<RefCell<Value>>;
pub type FunctionRef = Rc<FunctionProto>;
pub type ClosureRef = Rc<Closure>;

#[derive(Debug, Clone, Copy)]
pub enum UpvalueSpec {
    Local(u16),
    Parent(u16),
}

#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionProto {
    pub arity: u16,
    pub parameters: Vec<FunctionParameter>,
    pub chunk: Rc<Chunk>,
    pub upvalue_specs: Vec<UpvalueSpec>,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: FunctionRef,
    pub upvalues: Vec<CellRef>,
}

#[derive(Debug)]
pub struct CallFrame {
    pub closure: ClosureRef,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub cells: Option<Vec<Option<CellRef>>>,
    pub range_cursors: Vec<Option<RangeCursor>>,
    pub module: Option<ModuleRef>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct RangeCursor {
    pub current: i64,

    /*
     * Always exclusive.
     *
     * For:
     *
     *     0..10
     *     end = 10
     *
     *     0..=10
     *     end = 11
     */
    pub end: i64,
}