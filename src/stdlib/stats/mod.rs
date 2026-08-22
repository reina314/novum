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

    module.set_exported(
        "sum",
        Value::Builtin(
            descriptive::sum
        ),
    );

    module.set_exported(
        "mean",
        Value::Builtin(
            descriptive::mean
        ),
    );

    module.set_exported(
        "variance",
        Value::Builtin(
            descriptive::variance
        ),
    );

    module.set_exported(
        "std",
        Value::Builtin(
            descriptive::std
        ),
    );

    module.set_exported(
        "median",
        Value::Builtin(
            descriptive::median
        ),
    );

    module.set_exported(
        "quantile",
        Value::Builtin(
            descriptive::quantile
        ),
    );

    module.set_exported(
        "covariance",
        Value::Builtin(
            descriptive::covariance
        ),
    );

    module.set_exported(
        "pearson",
        Value::Builtin(
            descriptive::pearson
        ),
    );

    module.set_exported(
        "spearman",
        Value::Builtin(
            descriptive::spearman
        ),
    );

    module.set_exported(
        "one_sample_t",
        Value::Builtin(
            inferential::one_sample_t
        ),
    );

    module.set_exported(
        "welch_t",
        Value::Builtin(
            inferential::welch_t
        ),
    );

    module.set_exported(
        "paired_t",
        Value::Builtin(
            inferential::paired_t
        ),
    );

    module.set_exported(
        "mann_whitney",
        Value::Builtin(
            inferential::mann_whitney
        ),
    );

    module.set_exported(
        "anova",
        Value::Builtin(
            inferential::anova
        ),
    );

    module.set_exported(
        "kruskal_wallis",
        Value::Builtin(
            inferential::kruskal_wallis
        ),
    );

    module.set_exported(
        "chi_square",
        Value::Builtin(
            inferential::chi_square
        ),
    );

    module.set_exported(
        "chi_square_gof",
        Value::Builtin(
            inferential::chi_square_gof
        ),
    );

    module.set_exported(
        "chi_square_independence",
        Value::Builtin(
            inferential::chi_square_independence
        ),
    );

    module.set_exported(
        "mean_ci",
        Value::Builtin(
            inferential::mean_ci
        ),
    );

    Rc::new(
        RefCell::new(module)
    )
}