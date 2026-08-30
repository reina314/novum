use crate::runtime::ModuleRef;

pub mod builtin;
pub mod fs;
pub mod math;
pub mod process;
pub mod csv;
pub mod json;
pub mod linalg;
pub mod stats;

// For internal use
pub mod util;

pub use util::{
    encode_method_call,
    decode_method_call,
    encode_class_counts,
    decode_class_counts,
    encode_call_operand,
    decode_call_operand,
    is_self_pattern,
    result_err,
    result_ok,
    option_some,
    option_none,
};

/// Defines lazy stdlib module
pub fn load_module(
    name: &str,
) -> Option<ModuleRef> {
    match name {
        "fs" => Some(fs::module()),
        "math" => Some(math::module()),
        "process" => Some(process::module()),
        "csv" => Some(csv::module()),
        "json" => Some(json::module()),
        "linalg" => Some(linalg::module()),
        "stats" => Some(stats::module()),

        _ => None,
    }
}