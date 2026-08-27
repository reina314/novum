use crate::runtime::FunctionRef;

#[derive(Clone, Debug)]
pub enum PipelineSource {
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },

    Expression,
}

#[derive(Clone, Debug)]
pub enum PipelineStage {
    Map {
        function: FunctionRef,
    },

    Filter {
        function: FunctionRef,
    },

    Skip {
        count: usize,
    },

    Take {
        count: usize,
    },
}

#[derive(Clone, Debug)]
pub struct PipelineProgram {
    pub source: PipelineSource,
    pub stages: Vec<PipelineStage>,
}