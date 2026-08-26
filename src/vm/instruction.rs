#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Constant,

    Unit,

    Pop,
    Dup,

    LoadLocal,
    StoreLocal,
    ResetLocal,

    LoadUpvalue,
    StoreUpvalue,

    Closure,

    NewList,
    ListAppend,
    ListExtendRange,
    
    IndexGet,
    IndexSet,

    IteratorFrom,
    IteratorNext,

    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    MatMul,

    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,

    Neg,
    Not,

    Jump,
    JumpIfFalse,

    Call,
    Return,

    Halt,
}