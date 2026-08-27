#[derive(Clone, Copy, Debug)]
pub enum PipelineStepKind {
    Map,
    Filter,
    Skip,
    Take,
}

#[derive(Clone, Copy, Debug)]
pub struct PipelineStep {
    pub kind: PipelineStepKind,
    pub value: usize,
}

#[derive(Clone, Debug)]
pub struct PipelinePlan {
    pub steps: Vec<PipelineStep>,
    pub closure_count: usize,
}