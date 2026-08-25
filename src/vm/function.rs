use std::rc::Rc;

use super::chunk::Chunk;
use crate::runtime::Value;

pub type VmFunctionRef = Rc<VmFunction>;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub chunk: Rc<Chunk>,
    pub ip: usize,
    pub locals: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct VmFunction {
    pub arity: u16,
    pub chunk: Rc<Chunk>,
}