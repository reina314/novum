use crate::runtime::{
    ModuleRef,
    Value,
    Env,
};

use std::{
    collections::HashMap,
};

pub mod general;
pub mod math;
pub mod linalg;
pub mod csv;
pub mod stats;

/// Defines eager stdlib module
pub fn install_builtins(
    env: &Env
) {
    general::install_builtins(env);

    for (name, value) 
        in builtins() {
        env.define(name, value);
    }
}

/// Defines lazy stdlib module
pub fn load_module(
    name: &str,
) -> Option<ModuleRef> {
    match name {
        "stats" =>
            Some(stats::module()),

        "linalg" =>
            Some(linalg::module()),

        "math" =>
            Some(math::module()),

        "csv" =>
            Some(csv::module()),

        _ => None,
    }
}

fn builtins()
    -> HashMap<String, Value>
{
    let mut map = HashMap::new();

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


    //== Descriptive Statistics (stats/descriptive.rs) ==========
    // DEPRECATED and use `stats_module()` instead
    // This section is for backward compatibility only
    
    // EMPTY

    //========================================


    //== Inferential Statistics (stats/inferential.rs) ==========
    // DEPRECATED and use `stats_module()` instead
    // This section is for backward compatibility only
    
    // EMPTY

    //========================================

    map
}
