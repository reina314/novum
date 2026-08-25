use std::rc::Rc;

use super::chunk::Chunk;

pub type VmFunctionRef = Rc<VmFunction>;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub chunk: Rc<Chunk>,
    pub ip: usize,
    pub base: usize,
}

#[derive(Debug, Clone)]
pub struct VmFunction {
    pub arity: u16,
    pub chunk: Rc<Chunk>,
}