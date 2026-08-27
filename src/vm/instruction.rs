use crate::syntax::BinOp;

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
    CompoundAssignLocal,

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

    RangeInit,
    RangeNext,

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

#[derive(Debug, Clone, Copy)]
pub enum LocalBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl LocalBinaryOp {
    #[inline]
    pub fn from_binop(
        op: BinOp,
    ) -> Option<Self> {
        match op {
            BinOp::Add =>
                Some(Self::Add),

            BinOp::Sub =>
                Some(Self::Sub),

            BinOp::Mul =>
                Some(Self::Mul),

            BinOp::Div =>
                Some(Self::Div),

            BinOp::Mod =>
                Some(Self::Mod),

            BinOp::Pow =>
                Some(Self::Pow),

            _ =>
                None,
        }
    }

    #[inline]
    pub fn encode(
        self
    ) -> u32 {
        match self {
            Self::Add => 0,
            Self::Sub => 1,
            Self::Mul => 2,
            Self::Div => 3,
            Self::Mod => 4,
            Self::Pow => 5,
        }
    }

    #[inline]
    pub fn decode(
        value: u32
    ) -> Option<Self> {
        match value {
            0 => Some(Self::Add),
            1 => Some(Self::Sub),
            2 => Some(Self::Mul),
            3 => Some(Self::Div),
            4 => Some(Self::Mod),
            5 => Some(Self::Pow),
            _ => None,
        }
    }

    #[inline]
    pub fn encode_compound_assign(
        target_slot: u16,
        value_slot: u16,
        op: LocalBinaryOp,
    ) -> u32 {
        (op.encode() & 0x0f)
            | ((target_slot as u32) << 4)
            | ((value_slot as u32) << 18)
    }

    #[inline]
    pub fn decode_compound_assign(
        operand: u32,
    ) -> Option<(
        u16,
        u16,
        LocalBinaryOp,
    )> {
        let op =
            LocalBinaryOp::decode(
                operand & 0x0f
            )?;

        let target =
            ((operand >> 4) & 0x3fff)
                as u16;

        let value =
            ((operand >> 18) & 0x3fff)
                as u16;

        Some((
            target,
            value,
            op,
        ))
    }

}