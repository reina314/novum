#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Constant,

    Pop,

    LoadLocal,
    StoreLocal,

    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

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