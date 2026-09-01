use crate::runtime::{ExtensionRegistry, ModuleRef};

pub mod builtin;
pub mod csv;
pub mod fs;
pub mod json;
pub mod linalg;
pub mod math;
pub mod process;
pub mod stats;

// For internal use
pub mod util;

pub use util::{
    decode_call_operand, decode_class_counts, decode_method_call, encode_call_operand,
    encode_class_counts, encode_method_call, is_self_pattern, option_none, option_some, result_err,
    result_ok,
};

/// Defines lazy stdlib module
pub fn load_module(name: &str) -> Option<ModuleRef> {
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

pub fn extension_registry() -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new();

    math::register_extensions(&mut registry);
    stats::register_extensions(&mut registry);

    registry
}
