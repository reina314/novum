use std::{
    cell::RefCell,
    rc::Rc,
};

use super::Value;
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
pub struct FunctionProto {
    pub arity: u16,
    pub parameters: Vec<String>,
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
    pub cells: Vec<Option<CellRef>>,
    pub range_cursors: Vec<Option<RangeCursor>>,
}

#[derive(Debug, Clone, Copy)]
pub struct RangeCursor {
    pub current: i64,
    pub end: i64,
}