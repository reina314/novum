use crate::runtime::Value;
use super::{
    OpCode,
    Instruction,
    LocalBinaryOp,
    PipelineProgram,
};

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub local_count: usize,
    pub call_sites: Vec<CallSite>,
    pub pipelines: Vec<PipelineProgram>,
    pub range_loops: Vec<RangeLoop>,
    pub compound_assign_locals: Vec<CompoundAssignLocalSite>,
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
    pub fn add_call_site(
        &mut self,
        names: Vec<Option<String>>,
        method: Option<u32>,
    ) -> u32 {
        let index =
            self.call_sites.len();

        self.call_sites.push(
            CallSite {
                names,
                method,
            }
        );

        index as u32
    }

    pub fn add_pipeline(
        &mut self,
        pipeline: PipelineProgram,
    ) -> u32 {
        let index =
            self.pipelines.len();

        self.pipelines.push(
            pipeline
        );

        index as u32
    }

    pub fn add_range_loop(
        &mut self,
        range: RangeLoop,
    ) -> u32 {
        let index =
            self.range_loops.len();

        self.range_loops.push(
            range
        );

        index as u32
    }

    #[inline]
    pub fn add_compound_assign_local(
        &mut self,
        site: CompoundAssignLocalSite,
    ) -> u32 {
        let index =
            self.compound_assign_locals.len();

        self.compound_assign_locals.push(
            site
        );

        index as u32
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

impl Default for Chunk {
    fn default() -> Self {
        Self {
            code:
                Vec::new(),

            constants:
                Vec::new(),

            local_count:
                0,

            call_sites:
                Vec::new(),

            pipelines:
                Vec::new(),

            range_loops:
                Vec::new(),

            compound_assign_locals:
                Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CallSite {
    pub names: Vec<Option<String>>,
    pub method: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct RangeLoop {
    pub value_slot: u16,
    pub inclusive: bool,
    pub exit_ip: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CompoundAssignLocalSite {
    pub target_slot: u16,
    pub value_slot: u16,
    pub op: LocalBinaryOp,
}