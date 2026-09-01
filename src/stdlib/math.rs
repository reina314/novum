use crate::runtime::{ExtensionRegistry, Module, ModuleRef, ReceiverKind, Value};

use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy)]
enum ReceiverSpec {
    None,
    Numeric,
    Exact(ReceiverKind),
}

struct FunctionSpec {
    name: &'static str,
    function: fn(Vec<Value>) -> Result<Value, String>,
    receiver: ReceiverSpec,
}

fn function_specs() -> &'static [FunctionSpec] {
    &[
        FunctionSpec {
            name: "sqrt",
            function: sqrt,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "cbrt",
            function: cbrt,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "sin",
            function: sin,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "cos",
            function: cos,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "tan",
            function: tan,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "abs",
            function: abs,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "floor",
            function: floor,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "ceil",
            function: ceil,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "round",
            function: round,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "trunc",
            function: trunc,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "fract",
            function: fract,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "sign",
            function: sign,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "exp",
            function: exp,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "exp2",
            function: exp2,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "ln",
            function: ln,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "log",
            function: log,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "log2",
            function: log2,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "log10",
            function: log10,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "asin",
            function: asin,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "acos",
            function: acos,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "atan",
            function: atan,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "sinh",
            function: sinh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "cosh",
            function: cosh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "tanh",
            function: tanh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "asinh",
            function: asinh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "acosh",
            function: acosh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "atanh",
            function: atanh,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "pow",
            function: pow,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "atan2",
            function: atan2,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "hypot",
            function: hypot,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "min",
            function: min,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "max",
            function: max,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "clamp",
            function: clamp,
            receiver: ReceiverSpec::Numeric,
        },
        FunctionSpec {
            name: "pi",
            function: pi,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "e",
            function: e,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "tau",
            function: tau,
            receiver: ReceiverSpec::None,
        },
    ]
}

pub fn module() -> ModuleRef {
    let mut module = Module::new("math");

    for spec in function_specs() {
        module.set_exported(spec.name, Value::Builtin(spec.function));
    }

    Rc::new(RefCell::new(module))
}

pub fn register_extensions(registry: &mut ExtensionRegistry) {
    for spec in function_specs() {
        let value = Value::Builtin(spec.function);

        match spec.receiver {
            ReceiverSpec::None => {},

            ReceiverSpec::Numeric => {
                registry.register_numeric(spec.name, value);
            },

            ReceiverSpec::Exact(receiver) => {
                registry.register(receiver, spec.name, value);
            },
        }
    }
}

fn number(value: &Value) -> Result<f64, String> {
    match value {
        Value::Int(x) => Ok(*x as f64),
        Value::Float(x) => Ok(*x),

        other => Err(format!("expected numeric value, got {}", other.type_name())),
    }
}

fn unary<F>(args: Vec<Value>, name: &str, f: F) -> Result<Value, String>
where
    F: FnOnce(f64) -> f64,
{
    if args.len() != 1 {
        return Err(format!("{name}() expects exactly 1 argument"));
    }

    let x = number(&args[0])?;

    Ok(Value::Float(f(x)))
}

fn binary<F>(args: Vec<Value>, name: &str, f: F) -> Result<Value, String>
where
    F: FnOnce(f64, f64) -> f64,
{
    if args.len() != 2 {
        return Err(format!("{name}() expects exactly 2 arguments"));
    }

    let x = number(&args[0])?;
    let y = number(&args[1])?;

    Ok(Value::Float(f(x, y)))
}

// --------------------------------------------------
// Basic
// --------------------------------------------------

pub fn abs(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "abs", f64::abs)
}

pub fn sign(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "sign", f64::signum)
}

pub fn floor(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "floor", f64::floor)
}

pub fn ceil(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "ceil", f64::ceil)
}

pub fn round(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "round", f64::round)
}

pub fn trunc(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "trunc", f64::trunc)
}

pub fn fract(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "fract", f64::fract)
}

// --------------------------------------------------
// Powers / roots / exponentials / logarithms
// --------------------------------------------------

pub fn sqrt(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "sqrt", f64::sqrt)
}

pub fn cbrt(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "cbrt", f64::cbrt)
}

pub fn pow(args: Vec<Value>) -> Result<Value, String> {
    binary(args, "pow", f64::powf)
}

pub fn exp(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "exp", f64::exp)
}

pub fn exp2(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "exp2", f64::exp2)
}

pub fn ln(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "ln", f64::ln)
}

pub fn log(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "log", f64::ln)
}

pub fn log2(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "log2", f64::log2)
}

pub fn log10(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "log10", f64::log10)
}

// --------------------------------------------------
// Trigonometric
// --------------------------------------------------

pub fn sin(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "sin", f64::sin)
}

pub fn cos(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "cos", f64::cos)
}

pub fn tan(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "tan", f64::tan)
}

pub fn asin(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "asin", f64::asin)
}

pub fn acos(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "acos", f64::acos)
}

pub fn atan(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "atan", f64::atan)
}

pub fn atan2(args: Vec<Value>) -> Result<Value, String> {
    binary(args, "atan2", f64::atan2)
}

// --------------------------------------------------
// Hyperbolic
// --------------------------------------------------

pub fn sinh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "sinh", f64::sinh)
}

pub fn cosh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "cosh", f64::cosh)
}

pub fn tanh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "tanh", f64::tanh)
}

pub fn asinh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "asinh", f64::asinh)
}

pub fn acosh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "acosh", f64::acosh)
}

pub fn atanh(args: Vec<Value>) -> Result<Value, String> {
    unary(args, "atanh", f64::atanh)
}

// --------------------------------------------------
// Numeric utilities
// --------------------------------------------------

pub fn hypot(args: Vec<Value>) -> Result<Value, String> {
    binary(args, "hypot", f64::hypot)
}

pub fn min(args: Vec<Value>) -> Result<Value, String> {
    binary(args, "min", f64::min)
}

pub fn max(args: Vec<Value>) -> Result<Value, String> {
    binary(args, "max", f64::max)
}

pub fn clamp(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("clamp() expects exactly 3 arguments".into());
    }

    let x = number(&args[0])?;
    let min = number(&args[1])?;
    let max = number(&args[2])?;

    if min > max {
        return Err("clamp() requires min <= max".into());
    }

    Ok(Value::Float(x.clamp(min, max)))
}

// --------------------------------------------------
// Constants
// --------------------------------------------------

pub fn pi(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("pi() expects no arguments".into());
    }

    Ok(Value::Float(std::f64::consts::PI))
}

pub fn e(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("e() expects no arguments".into());
    }

    Ok(Value::Float(std::f64::consts::E))
}

pub fn tau(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("tau() expects no arguments".into());
    }

    Ok(Value::Float(std::f64::consts::TAU))
}
