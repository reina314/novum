use crate::{
    runtime::FunctionRef,
    error::{
        Error,
        ErrorKind,
    }
};

#[derive(Clone, Debug)]
pub enum PipelineSource {
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
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

impl PipelineProgram {
    pub fn capacity_upper_bound(
        &self,
    ) -> Result<usize, Error> {
        let PipelineSource::Range {
            start,
            end,
            inclusive,
        } =
            self.source;

        let end =
            if inclusive {
                end.checked_add(1)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Overflow,
                            "inclusive range endpoint overflow",
                            None,
                        )
                    })?
            } else {
                end
            };

        if end <= start {
            return Ok(0);
        }

        let count =
            end
                .checked_sub(start)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "pipeline range size overflow",
                        None,
                    )
                })?;

        let mut capacity =
            usize::try_from(
                count
            )
            .map_err(|_| {
                Error::new(
                    ErrorKind::Overflow,
                    "pipeline range is too large",
                    None,
                )
            })?;

        for stage in
            &self.stages
        {
            if let PipelineStage::Take {
                count,
            } = stage
            {
                capacity =
                    capacity.min(
                        *count
                    );
            }
        }

        Ok(capacity)
    }
}

#[derive(Clone, Copy)]
pub enum PipelineState {
    None,
    Skip(usize),
    Take(usize),
}

