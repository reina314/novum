use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    runtime::UpvalueSpec,
};

#[derive(Clone, Debug)]
pub enum PipelineSource {
    /*
     * Compile-time constant range.
     */
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },

    /*
     * Runtime-evaluated integer range.
     *
     * The expressions are evaluated once, before the
     * fused iteration begins.
     */
    DynamicRange {
        start: PipelineExpr,
        end: PipelineExpr,
        inclusive: bool,

        /*
         * Captures belong to the source itself, rather than
         * to an individual map/filter stage.
         */
        captures: Vec<UpvalueSpec>,

        /*
         * range(n) has historically rejected negative n.
         * Keep that behavior when the source originated from
         * the one-argument builtin.
         */
        require_non_negative_end: bool,
    },
}

#[derive(Clone, Debug)]
pub enum PipelineExpr {
    Input,

    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),

    Add(
        Box<Self>,
        Box<Self>,
    ),

    Sub(
        Box<Self>,
        Box<Self>,
    ),

    Mul(
        Box<Self>,
        Box<Self>,
    ),

    Div(
        Box<Self>,
        Box<Self>,
    ),

    Mod(
        Box<Self>,
        Box<Self>,
    ),

    Pow(
        Box<Self>,
        Box<Self>,
    ),

    Eq(
        Box<Self>,
        Box<Self>,
    ),

    Neq(
        Box<Self>,
        Box<Self>,
    ),

    Lt(
        Box<Self>,
        Box<Self>,
    ),

    Leq(
        Box<Self>,
        Box<Self>,
    ),

    Gt(
        Box<Self>,
        Box<Self>,
    ),

    Geq(
        Box<Self>,
        Box<Self>,
    ),

    Neg(
        Box<Self>,
    ),

    Not(
        Box<Self>,
    ),

    Capture(u16),
}

#[derive(Clone, Debug)]
pub enum PipelineStage {
    Map {
        expr: PipelineExpr,
        captures: Vec<UpvalueSpec>,
    },

    Filter {
        expr: PipelineExpr,
        captures: Vec<UpvalueSpec>,
    },

    Skip {
        count: usize,
    },

    Take {
        count: usize,
    },
}

/*
 * ============================================================
 * Specialized integer pipeline IR
 * ============================================================
 */

#[derive(Clone, Debug)]
pub enum IntPipelineExpr {
    Input,

    Const(i64),

    Capture(u16),

    Add(
        Box<Self>,
        Box<Self>,
    ),

    Sub(
        Box<Self>,
        Box<Self>,
    ),

    Mul(
        Box<Self>,
        Box<Self>,
    ),

    Div(
        Box<Self>,
        Box<Self>,
    ),

    Mod(
        Box<Self>,
        Box<Self>,
    ),

    Neg(
        Box<Self>,
    ),
}

#[derive(Clone, Debug)]
pub enum IntPipelinePredicate {
    Eq(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),

    Neq(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),

    Lt(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),

    Leq(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),

    Gt(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),

    Geq(
        Box<IntPipelineExpr>,
        Box<IntPipelineExpr>,
    ),
}

#[derive(Clone, Debug)]
pub enum IntPipelineStage {
    Map(
        IntPipelineExpr,
    ),

    Filter(
        IntPipelinePredicate,
    ),

    Skip(
        usize,
    ),

    Take(
        usize,
    ),
}

/*
 * ============================================================
 * Pipeline execution plan
 * ============================================================
 */

#[derive(Clone, Debug)]
pub enum PipelinePlan {
    Generic,

    IntRange {
        stages:
            Vec<IntPipelineStage>,
    },
}

#[derive(Clone, Debug)]
pub struct PipelineProgram {
    pub source:
        PipelineSource,

    /*
     * Canonical semantic representation.
     */
    pub stages:
        Vec<PipelineStage>,

    /*
     * Specialized execution plan.
     *
     * `Generic` remains the semantic fallback.
     */
    pub plan:
        PipelinePlan,
}

impl PipelineProgram {
    pub fn capacity_upper_bound(
        &self,
    ) -> Result<usize> {
        match self.source {
            PipelineSource::Range {
                start,
                end,
                inclusive,
            } => {
                Self::capacity_upper_bound_for_range(
                    start,
                    end,
                    inclusive,
                    &self.stages,
                )
            }

            PipelineSource::DynamicRange {
                ..
            } => {
                /*
                 * Bounds are not known until runtime.
                 */
                Ok(0)
            }
        }
    }

    pub fn capacity_upper_bound_for_range(
        start: i64,
        end: i64,
        inclusive: bool,
        stages: &[PipelineStage],
    ) -> Result<usize> {
        let normalized_end =
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

        if normalized_end <= start {
            return Ok(0);
        }

        let count =
            normalized_end
                .checked_sub(start)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Overflow,
                        "pipeline range size overflow",
                        None,
                    )
                })?;

        let mut capacity =
            usize::try_from(count)
                .map_err(|_| {
                    Error::new(
                        ErrorKind::Overflow,
                        "pipeline range is too large",
                        None,
                    )
                })?;

        for stage in stages {
            if let PipelineStage::Take {
                count,
            } = stage
            {
                capacity =
                    capacity.min(*count);
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