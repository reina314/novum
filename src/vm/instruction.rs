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

    NewTuple,

    NewList,
    ListAppend,
    ListExtendRange,

    NewRange,
    
    IndexGet,
    IndexSet,

    FieldGet,
    EnumFieldGet,
    FieldSet,

    IteratorFrom,
    IteratorNext,

    MatchTuple,
    MatchList,
    MatchEnum,
    PatternFail,

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

    InvokeMethod,

    Call,
    Return,

    Halt,
}

pub fn encode_method_call(
    method_index: u16,
    argc: u16,
) -> u32 {
    ((method_index as u32) << 16)
        | argc as u32
}

pub fn decode_method_call(
    operand: u32,
) -> (u16, u16) {
    (
        (operand >> 16) as u16,
        (operand & 0xffff) as u16,
    )
}