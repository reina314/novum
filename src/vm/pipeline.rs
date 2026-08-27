#[derive(Clone, Copy, Debug)]
pub enum PipelineStep {
    Map {
        function_constant: u32,
    },

    Filter {
        function_constant: u32,
    },

    Skip {
        count: usize,
    },

    Take {
        count: usize,
    },
}

#[derive(Clone, Debug)]
pub struct PipelinePlan {
    pub steps: Vec<PipelineStep>,
}