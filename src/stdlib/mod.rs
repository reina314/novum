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
pub mod csv;
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

        "csv" =>
            Some(csv_module()),

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
        "chi_square",
        Value::Builtin(
            statistics::inferential::chi_square
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
        "sign",
        Value::Builtin(math::sign),
    );

    module.set(
        "floor",
        Value::Builtin(math::floor),
    );

    module.set(
        "ceil",
        Value::Builtin(math::ceil),
    );

    module.set(
        "round",
        Value::Builtin(math::round),
    );

    module.set(
        "trunc",
        Value::Builtin(math::trunc),
    );

    module.set(
        "fract",
        Value::Builtin(math::fract),
    );

    module.set(
        "sqrt",
        Value::Builtin(math::sqrt),
    );

    module.set(
        "cbrt",
        Value::Builtin(math::cbrt),
    );

    module.set(
        "pow",
        Value::Builtin(math::pow),
    );

    module.set(
        "exp",
        Value::Builtin(math::exp),
    );

    module.set(
        "exp2",
        Value::Builtin(math::exp2),
    );

    module.set(
        "ln",
        Value::Builtin(math::ln),
    );

    module.set(
        "log",
        Value::Builtin(math::log),
    );

    module.set(
        "log2",
        Value::Builtin(math::log2),
    );

    module.set(
        "log10",
        Value::Builtin(math::log10),
    );

    module.set(
        "sin",
        Value::Builtin(math::sin),
    );

    module.set(
        "cos",
        Value::Builtin(math::cos),
    );

    module.set(
        "tan",
        Value::Builtin(math::tan),
    );

    module.set(
        "asin",
        Value::Builtin(math::asin),
    );

    module.set(
        "acos",
        Value::Builtin(math::acos),
    );

    module.set(
        "atan",
        Value::Builtin(math::atan),
    );

    module.set(
        "atan2",
        Value::Builtin(math::atan2),
    );

    module.set(
        "sinh",
        Value::Builtin(math::sinh),
    );

    module.set(
        "cosh",
        Value::Builtin(math::cosh),
    );

    module.set(
        "tanh",
        Value::Builtin(math::tanh),
    );

    module.set(
        "asinh",
        Value::Builtin(math::asinh),
    );

    module.set(
        "acosh",
        Value::Builtin(math::acosh),
    );

    module.set(
        "atanh",
        Value::Builtin(math::atanh),
    );

    module.set(
        "hypot",
        Value::Builtin(math::hypot),
    );

    module.set(
        "min",
        Value::Builtin(math::min),
    );

    module.set(
        "max",
        Value::Builtin(math::max),
    );

    module.set(
        "clamp",
        Value::Builtin(math::clamp),
    );

    module.set(
        "pi",
        Value::Builtin(math::pi),
    );

    module.set(
        "e",
        Value::Builtin(math::e),
    );

    module.set(
        "tau",
        Value::Builtin(math::tau),
    );

    Rc::new(
        RefCell::new(module)
    )
}

fn csv_module() -> ModuleRef {
    let mut module =
        Module::new("csv");

    module.set(
        "read",
        Value::Builtin(
            csv::read
        ),
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
        "typeof".into(),
        Value::Builtin(general::r#typeof),
    );

    map.insert(
        "iter".into(),
        Value::Builtin(general::iter),
    );

    map.insert(
        "range".into(),
        Value::Builtin(general::range),
    );

    map.insert(
        "len".into(),
        Value::Builtin(general::len),
    );

    map.insert(
        "is_null".into(),
        Value::Builtin(general::is_null),
    );

    map.insert(
        "is_type".into(),
        Value::Builtin(general::is_type),
    );

    map.insert(
        "assert".into(),
        Value::Builtin(general::assert),
    );

    map.insert(
        "panic".into(),
        Value::Builtin(general::panic),
    );

    map.insert(
        "input".into(),
        Value::Builtin(general::input),
    );

    map.insert(
        "read".into(),
        Value::Builtin(general::read),
    );

    map.insert(
        "write".into(),
        Value::Builtin(general::write),
    );

    map.insert(
        "append".into(),
        Value::Builtin(general::append),
    );

    map.insert(
        "str".into(),
        Value::Builtin(general::str),
    );

    map.insert(
        "int".into(),
        Value::Builtin(general::int),
    );

    map.insert(
        "float".into(),
        Value::Builtin(general::float),
    );

    map.insert(
        "args".into(),
        Value::Builtin(general::args),
    );

    map.insert(
        "env".into(),
        Value::Builtin(general::env),
    );

    map.insert(
        "cwd".into(),
        Value::Builtin(general::cwd),
    );

    map.insert(
        "sleep".into(),
        Value::Builtin(general::sleep),
    );

    map.insert(
        "random".into(),
        Value::Builtin(general::random),
    );

    map.insert(
        "randint".into(),
        Value::Builtin(general::randint),
    );
    //========================================


    //== Math (math.rs) ======================
    // DEPRECATED and use `math_module()` instead
    // This section is for backward compatibility only
    map.insert(
        "sqrt".into(),
        Value::Builtin(math::sqrt),
    );
    //========================================


    //== Linear Algebra (linalg.rs) ==========
    // DEPRECATED and use `linalg_module()` instead
    // This section is for backward compatibility only
    map.insert(
        "matrix".into(),
        Value::Builtin(linalg::matrix),
    );
    //========================================


    //== Descriptive Statistics (statistics/descriptive.rs) ==========
    // DEPRECATED and use `stats_module()` instead
    // This section is for backward compatibility only
    
    // EMPTY

    //========================================


    //== Inferential Statistics (statistics/inferential.rs) ==========
    // DEPRECATED and use `stats_module()` instead
    // This section is for backward compatibility only
    
    // EMPTY

    //========================================

    map
}
