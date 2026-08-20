use crate::runtime::{
    Module,
    ModuleRef,
    Value,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

pub mod general;
pub mod math;
pub mod linalg;
pub mod statistics;

pub fn load_module(
    name: &str,
) -> Option<ModuleRef> {
    match name {
        "stats" =>
            Some(stats_module()),

        "linalg" =>
            Some(linalg_module()),

        "math" =>
            Some(math_module()),

        _ => None,
    }
}

fn stats_module() -> ModuleRef {
    let mut module =
        Module::new("stats");

    module.set(
        "sum",
        Value::Builtin(
            statistics::descriptive::sum
        ),
    );

    module.set(
        "mean",
        Value::Builtin(
            statistics::descriptive::mean
        ),
    );

    module.set(
        "variance",
        Value::Builtin(
            statistics::descriptive::variance
        ),
    );

    module.set(
        "std",
        Value::Builtin(
            statistics::descriptive::std
        ),
    );

    module.set(
        "median",
        Value::Builtin(
            statistics::descriptive::median
        ),
    );

    module.set(
        "quantile",
        Value::Builtin(
            statistics::descriptive::quantile
        ),
    );

    module.set(
        "covariance",
        Value::Builtin(
            statistics::descriptive::covariance
        ),
    );

    module.set(
        "pearson",
        Value::Builtin(
            statistics::descriptive::pearson
        ),
    );

    module.set(
        "spearman",
        Value::Builtin(
            statistics::descriptive::spearman
        ),
    );

    module.set(
        "one_sample_t",
        Value::Builtin(
            statistics::inferential::one_sample_t
        ),
    );

    module.set(
        "welch_t",
        Value::Builtin(
            statistics::inferential::welch_t
        ),
    );

    module.set(
        "paired_t",
        Value::Builtin(
            statistics::inferential::paired_t
        ),
    );

    module.set(
        "mann_whitney",
        Value::Builtin(
            statistics::inferential::mann_whitney
        ),
    );

    module.set(
        "anova",
        Value::Builtin(
            statistics::inferential::anova
        ),
    );

    module.set(
        "kruskal_wallis",
        Value::Builtin(
            statistics::inferential::kruskal_wallis
        ),
    );

    module.set(
        "chi_square_gof",
        Value::Builtin(
            statistics::inferential::chi_square_gof
        ),
    );

    module.set(
        "chi_square_independence",
        Value::Builtin(
            statistics::inferential::chi_square_independence
        ),
    );

    module.set(
        "mean_ci",
        Value::Builtin(
            statistics::inferential::mean_ci
        ),
    );

    Rc::new(
        RefCell::new(module)
    )
}

fn linalg_module() -> ModuleRef {
    let mut module =
        Module::new("linalg");

    module.set(
        "matrix",
        Value::Builtin(
            linalg::matrix
        ),
    );

    module.set(
        "transpose",
        Value::Builtin(
            linalg::transpose
        ),
    );

    module.set(
        "det",
        Value::Builtin(
            linalg::det
        ),
    );

    module.set(
        "inverse",
        Value::Builtin(
            linalg::inverse
        ),
    );

    module.set(
        "shape",
        Value::Builtin(
            linalg::shape
        ),
    );

    module.set(
        "rows",
        Value::Builtin(
            linalg::rows
        ),
    );

    module.set(
        "cols",
        Value::Builtin(
            linalg::cols
        ),
    );

    module.set(
        "linear_regression",
        Value::Builtin(
            linalg::linear_regression
        ),
    );

    Rc::new(
        RefCell::new(module)
    )
}

fn math_module() -> ModuleRef {
    let mut module =
        Module::new("math");

    module.set(
        "sqrt",
        Value::Builtin(math::sqrt),
    );

    module.set(
        "abs",
        Value::Builtin(math::abs),
    );

    module.set(
        "exp",
        Value::Builtin(math::exp),
    );

    module.set(
        "log",
        Value::Builtin(math::log),
    );

    Rc::new(
        RefCell::new(module)
    )
}

pub fn builtins()
    -> HashMap<String, Value>
{
    let mut map = HashMap::new();

    //== General (general.rs) ================
    map.insert(
        "print".into(),
        Value::Builtin(general::print),
    );

    map.insert(
        "iter".into(),
        Value::Builtin(general::iter),
    );
    //========================================


    //== Math (math.rs) ======================
    map.insert(
        "sqrt".into(),
        Value::Builtin(math::sqrt),
    );
    map.insert(
        "abs".into(),
        Value::Builtin(math::abs),
    );
    map.insert(
        "exp".into(),
        Value::Builtin(math::exp),
    );
    map.insert(
        "log".into(),
        Value::Builtin(math::log),
    );
    //========================================


    //== Linear Algebra (linalg.rs) ==========
    map.insert(
        "matrix".into(),
        Value::Builtin(linalg::matrix),
    );

    map.insert(
        "transpose".into(),
        Value::Builtin(linalg::transpose),
    );

    map.insert(
        "det".into(),
        Value::Builtin(linalg::det),
    );

    map.insert(
        "inverse".into(),
        Value::Builtin(linalg::inverse),
    );

    map.insert(
        "shape".into(),
        Value::Builtin(linalg::shape),
    );
    
    map.insert(
        "rows".into(),
        Value::Builtin(linalg::rows),
    );

    map.insert(
        "cols".into(),
        Value::Builtin(linalg::cols),
    );

    map.insert(
        "linear_regression".into(),
        Value::Builtin(linalg::linear_regression),
    );
    //========================================


    //== Descriptive Statistics (statistics/descriptive.rs) ==========
    map.insert(
        "sum".into(),
        Value::Builtin(statistics::descriptive::sum),
    );

    map.insert(
        "mean".into(),
        Value::Builtin(statistics::descriptive::mean),
    );

    map.insert(
        "variance".into(),
        Value::Builtin(statistics::descriptive::variance),
    );

    map.insert(
        "std".into(),
        Value::Builtin(statistics::descriptive::std),
    );

    map.insert(
        "median".into(),
        Value::Builtin(statistics::descriptive::median),
    );

    map.insert(
        "min".into(),
        Value::Builtin(statistics::descriptive::min),
    );

    map.insert(
        "max".into(),
        Value::Builtin(statistics::descriptive::max),
    );

    map.insert(
        "quantile".into(),
        Value::Builtin(statistics::descriptive::quantile),
    );

    map.insert(
        "covariance".into(),
        Value::Builtin(statistics::descriptive::covariance),
    );

    map.insert(
        "pearson".into(),
        Value::Builtin(statistics::descriptive::pearson),
    );

    map.insert(
        "spearman".into(),
        Value::Builtin(statistics::descriptive::spearman),
    );
    //========================================


    //== Inferential Statistics (statistics/inferential.rs) ==========
    map.insert(
        "mean_ci".into(),
        Value::Builtin(statistics::inferential::mean_ci),
    );
    
    map.insert(
        "one_sample_t".into(),
        Value::Builtin(statistics::inferential::one_sample_t),
    );

    map.insert(
        "paired_t".into(),
        Value::Builtin(statistics::inferential::paired_t),
    );

    map.insert(
        "welch_t".into(),
        Value::Builtin(statistics::inferential::welch_t),
    );

    map.insert(
        "mann_whitney".into(),
        Value::Builtin(statistics::inferential::mann_whitney),
    );

    map.insert(
        "chi_square_gof".into(),
        Value::Builtin(statistics::inferential::chi_square_gof),
    );

    map.insert(
        "chi_square_independence".into(),
        Value::Builtin(statistics::inferential::chi_square_independence),
    );

    map.insert(
        "anova".into(),
        Value::Builtin(statistics::inferential::anova),
    );

    map.insert(
        "kruskal_wallis".into(),
        Value::Builtin(statistics::inferential::kruskal_wallis),
    );

    map
}
