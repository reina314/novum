use crate::runtime::Value;
use super::instruction::OpCode;

#[derive(Debug, Clone)]
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

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<Instruction>,
    pub constants: Vec<Value>,
}

impl Chunk {
    #[inline]
    pub fn add_constant(
        &mut self,
        value: Value,
    ) -> u32 {
        let index =
            self.constants.len() as u32;

        self.constants.push(value);

        index
    }

    #[inline]
    pub fn emit(
        &mut self,
        opcode: OpCode,
    ) -> usize {
        let index =
            self.code.len();

        self.code.push(
            Instruction::simple(
                opcode
            )
        );

        index
    }

    #[inline]
    pub fn emit_operand(
        &mut self,
        opcode: OpCode,
        operand: u32,
    ) -> usize {
        let index =
            self.code.len();

        self.code.push(
            Instruction::new(
                opcode,
                operand,
            )
        );

        index
    }

    #[inline]
    pub fn patch_operand(
        &mut self,
        index: usize,
        operand: u32,
    ) {
        self.code[index].operand =
            operand;
    }
}