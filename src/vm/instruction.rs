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
    NewStruct,
    NewClass,
    NewRange,
    
    ListAppend,
    ListExtendRange,

    MatchTuple,
    MatchEnum,
    MatchList,
    MatchStruct,
    PatternFail,

    IndexGet,
    FieldGet,
    EnumFieldGet,
    StructFieldGet,

    IndexSet,
    FieldSet,

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

    InvokeMethod,

    Call,
    Return,

    Halt,
}