#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: u32,
}

impl Instruction {
    #[inline]
    pub fn new(
        opcode: OpCode,
        operand: u32,
    ) -> Self {
        Self {
            opcode,
            operand,
        }
    }

    #[inline]
    pub fn simple(
        opcode: OpCode,
    ) -> Self {
        Self {
            opcode,
            operand: 0,
        }
    }
}


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

    FusedPipeline,

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
    JumpIfTrue,
    JumpIfFalse,

    InvokeMethod,

    Call,
    Return,

    Halt,
}