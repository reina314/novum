use std::{cell::RefCell, collections::HashMap, process::Command, rc::Rc};

use crate::{
    runtime::{ExtensionRegistry, List, Module, ModuleRef, PathValue, ReceiverKind, Value},
    stdlib::{option_none, option_some, result_err, result_ok},
};

#[derive(Clone, Copy)]
enum ReceiverSpec {
    None,
    Str,
}

struct FunctionSpec {
    name: &'static str,
    function: fn(Vec<Value>) -> Result<Value, String>,
    receiver: ReceiverSpec,
}

fn function_specs() -> &'static [FunctionSpec] {
    &[
        FunctionSpec {
            name: "args",
            function: args,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "env",
            function: env,
            receiver: ReceiverSpec::Str,
        },
        FunctionSpec {
            name: "cwd",
            function: cwd,
            receiver: ReceiverSpec::None,
        },
        FunctionSpec {
            name: "set_env",
            function: set_env,
            receiver: ReceiverSpec::Str,
        },
        FunctionSpec {
            name: "run",
            function: run,
            receiver: ReceiverSpec::Str,
        },
    ]
}

pub fn register_extensions(registry: &mut ExtensionRegistry) {
    for spec in function_specs() {
        let ReceiverSpec::Str = spec.receiver else {
            continue;
        };

        registry.register(ReceiverKind::Str, spec.name, Value::Builtin(spec.function));
    }
}

pub fn module() -> ModuleRef {
    let mut module = Module::new("process");

    for spec in function_specs() {
        module.set_exported(spec.name, Value::Builtin(spec.function));
    }

    Rc::new(RefCell::new(module))
}

pub fn args(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("process.args() expects no arguments".into());
    }

    let values = std::env::args()
        .skip(1)
        .map(|arg| Value::Str(Rc::new(arg)))
        .collect::<Vec<_>>();

    Ok(Value::List(List::new(values)))
}

pub fn env(mut args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("process.env() expects exactly 1 argument".into());
    }

    let name = match args.remove(0) {
        Value::Str(name) => name,

        other => {
            return Err(format!(
                "process.env() expects Str, got {}",
                other.type_name()
            ))
        },
    };

    match std::env::var(name.as_ref()) {
        Ok(value) => Ok(option_some(Value::Str(Rc::new(value)))),

        Err(std::env::VarError::NotPresent) => Ok(option_none()),

        Err(error) => Err(format!("failed to read environment variable: {}", error)),
    }
}

pub fn cwd(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("process.cwd() expects no arguments".into());
    }

    match std::env::current_dir() {
        Ok(path) => Ok(result_ok(Value::Path(Rc::new(PathValue::new(path))))),

        Err(error) => Ok(result_err(format!(
            "failed to get current directory: {}",
            error
        ))),
    }
}

pub fn set_env(mut args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("process.set_env() expects exactly 2 arguments".into());
    }

    let name = match args.remove(0) {
        Value::Str(value) => value,

        other => {
            return Err(format!(
                "process.set_env() expects first argument as Str, got {}",
                other.type_name()
            ))
        },
    };

    let value = match args.remove(0) {
        Value::Str(value) => value,

        other => {
            return Err(format!(
                "process.set_env() expects second argument as Str, got {}",
                other.type_name()
            ))
        },
    };

    // concurrency might cause race condition
    std::env::set_var(name.as_ref(), value.as_ref());

    Ok(result_ok(Value::Unit))
}

pub fn run(mut args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("process.run() expects exactly 2 arguments".into());
    }

    let command = match args.remove(0) {
        Value::Str(command) => command,

        other => {
            return Err(format!(
                "process.run() expects command as Str, got {}",
                other.type_name()
            ))
        },
    };

    let argv = expect_string_list(args.remove(0), "process.run()")?;

    let argv = collect_string_args(&argv, "process.run()")?;

    let output = match Command::new(command.as_ref()).args(&argv).output() {
        Ok(output) => output,

        Err(error) => {
            return Ok(result_err(format!(
                "failed to execute '{}': {}",
                command, error
            )))
        },
    };

    let status = output.status.code().unwrap_or(-1);

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok(result_ok(make_process_result(
        status as i64,
        stdout,
        stderr,
    )))
}

// Helper for `run()`
fn expect_string_list(value: Value, function: &str) -> Result<List, String> {
    match value {
        Value::List(list) => Ok(list),

        other => Err(format!(
            "{} expects List, got {}",
            function,
            other.type_name()
        )),
    }
}

fn collect_string_args(list: &List, function: &str) -> Result<Vec<String>, String> {
    let values = list.iter_cloned();

    let mut result = Vec::with_capacity(values.len());

    for value in values {
        match value {
            Value::Str(value) => result.push(value.as_ref().clone()),

            other => {
                return Err(format!(
                    "{} expects List<Str>, got element of type {}",
                    function,
                    other.type_name()
                ))
            },
        }
    }

    Ok(result)
}

fn make_process_result(status: i64, stdout: String, stderr: String) -> Value {
    let mut result = HashMap::new();

    result.insert("status".to_string(), Value::Int(status));

    result.insert("stdout".to_string(), Value::Str(Rc::new(stdout)));

    result.insert("stderr".to_string(), Value::Str(Rc::new(stderr)));

    Value::Dict(Rc::new(RefCell::new(result)))
}
