use crate::runtime::{
    Module,
    ModuleRef,
    Value,
};

use std::{
    cell::RefCell,
    rc::Rc,
};

pub mod descriptive;
pub mod inferential;
pub mod distribution;
pub mod util;

pub fn module() -> ModuleRef {
    let mut module =
        Module::new("stats");

    module.set(
        "sum",
        Value::Builtin(
            descriptive::sum
        ),
    );

    module.set(
        "mean",
        Value::Builtin(
            descriptive::mean
        ),
    );

    module.set(
        "variance",
        Value::Builtin(
            descriptive::variance
        ),
    );

    module.set(
        "std",
        Value::Builtin(
            descriptive::std
        ),
    );

    module.set(
        "median",
        Value::Builtin(
            descriptive::median
        ),
    );

    module.set(
        "quantile",
        Value::Builtin(
            descriptive::quantile
        ),
    );

    module.set(
        "covariance",
        Value::Builtin(
            descriptive::covariance
        ),
    );

    module.set(
        "pearson",
        Value::Builtin(
            descriptive::pearson
        ),
    );

    module.set(
        "spearman",
        Value::Builtin(
            descriptive::spearman
        ),
    );

    module.set(
        "one_sample_t",
        Value::Builtin(
            inferential::one_sample_t
        ),
    );

    module.set(
        "welch_t",
        Value::Builtin(
            inferential::welch_t
        ),
    );

    module.set(
        "paired_t",
        Value::Builtin(
            inferential::paired_t
        ),
    );

    module.set(
        "mann_whitney",
        Value::Builtin(
            inferential::mann_whitney
        ),
    );

    module.set(
        "anova",
        Value::Builtin(
            inferential::anova
        ),
    );

    module.set(
        "kruskal_wallis",
        Value::Builtin(
            inferential::kruskal_wallis
        ),
    );

    module.set(
        "chi_square",
        Value::Builtin(
            inferential::chi_square
        ),
    );

    module.set(
        "chi_square_gof",
        Value::Builtin(
            inferential::chi_square_gof
        ),
    );

    module.set(
        "chi_square_independence",
        Value::Builtin(
            inferential::chi_square_independence
        ),
    );

    module.set(
        "mean_ci",
        Value::Builtin(
            inferential::mean_ci
        ),
    );

    Rc::new(
        RefCell::new(module)
    )
}