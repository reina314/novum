use crate::runtime::Value;
use std::collections::HashMap;

pub mod general;
pub mod math;
pub mod linalg;
pub mod statistics;

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
