use std::{
    cell::RefCell,
    rc::Rc,
};

use crate::runtime::{
    Module,
    ModuleRef,
    Value,
    EnumValue,
    Object,
};

pub fn module() -> ModuleRef {
    let mut module =
        Module::new("process");

    module.set_exported(
        "args",
        Value::Builtin(args),
    );

    module.set_exported(
        "env",
        Value::Builtin(env),
    );

    module.set_exported(
        "cwd",
        Value::Builtin(cwd),
    );

    module.set_exported(
        "set_env",
        Value::Builtin(set_env),
    );

    module.set_exported(
        "run",
        Value::Builtin(run),
    );

    Rc::new(
        RefCell::new(
            module
        )
    )
}


pub fn args(
    args: Vec<Value>,
) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(
            "process.args() expects no arguments"
                .into()
        );
    }

    let values =
        std::env::args()
            .skip(1)
            .map(|arg| {
                Value::Str(
                    Rc::new(arg)
                )
            })
            .collect::<Vec<_>>();

    Ok(
        Value::List(
            Rc::new(
                RefCell::new(values)
            )
        )
    )
}

pub fn env(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "process.env() expects exactly 1 argument"
                .into()
        );
    }

    let name =
        match args.remove(0) {
            Value::Str(name) =>
                name,

            other => {
                return Err(
                    format!(
                        "process.env() expects Str, got {}",
                        other.type_name()
                    )
                );
            }
        };

    match std::env::var(
        name.as_ref()
    ) {
        Ok(value) =>
            Ok(
                option_some(
                    Value::Str(
                        Rc::new(value)
                    )
                )
            ),

        Err(std::env::VarError::NotPresent) =>
            Ok(
                option_none()
            ),

        Err(error) =>
            Err(
                format!(
                    "failed to read environment variable: {}",
                    error
                )
            ),
    }
}

pub fn cwd(
    args: Vec<Value>,
) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(
            "process.cwd() expects no arguments"
                .into()
        );
    }

    match std::env::current_dir() {
        Ok(path) =>
            Ok(
                result_ok(
                    Value::Str(
                        Rc::new(
                            path.to_string_lossy()
                                .into_owned()
                        )
                    )
                )
            ),

        Err(error) =>
            Ok(
                result_err(
                    format!(
                        "failed to get current directory: {}",
                        error
                    )
                )
            ),
    }
}

pub fn set_env(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "process.set_env() expects exactly 2 arguments"
                .into()
        );
    }

    let name =
        match args.remove(0) {
            Value::Str(name) =>
                name,

            other => {
                return Err(
                    format!(
                        "process.set_env() expects first argument as Str, got {}",
                        other.type_name()
                    )
                );
            }
        };

    let value =
        match args.remove(0) {
            Value::Str(value) =>
                value,

            other => {
                return Err(
                    format!(
                        "process.set_env() expects second argument as Str, got {}",
                        other.type_name()
                    )
                );
            }
        };

    // This may cause conflicts when multiple threads are trying to set the same environment variable at the same time.
    match std::env::set_var(
        name.as_ref(),
        value.as_ref(),
    ) {
        () =>
            Ok(
                result_ok(
                    Value::Unit
                )
            ),
    }
}

pub fn run(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "process.run() expects exactly 2 arguments"
                .into()
        );
    }

    let command =
        match args.remove(0) {
            Value::Str(command) =>
                command,

            other => {
                return Err(
                    format!(
                        "process.run() expects command as Str, got {}",
                        other.type_name()
                    )
                );
            }
        };

    let argv =
        match args.remove(0) {
            Value::List(list) =>
                list,

            other => {
                return Err(
                    format!(
                        "process.run() expects arguments as List, got {}",
                        other.type_name()
                    )
                );
            }
        };

    let values =
        argv.borrow();

    let mut command_builder =
        std::process::Command::new(
            command.as_ref()
        );

    for value in values.iter() {
        let argument =
            match value {
                Value::Str(value) =>
                    value.as_str(),

                other => {
                    return Err(
                        format!(
                            "process.run() arguments must be Str, got {}",
                            other.type_name()
                        )
                    );
                }
            };

        command_builder.arg(argument);
    }

    let output =
        match command_builder.output() {
            Ok(output) =>
                output,

            Err(error) =>
                return Ok(
                    result_err(
                        format!(
                            "failed to execute process: {}",
                            error
                        )
                    )
                ),
        };

    let status =
        output
            .status
            .code()
            .unwrap_or(-1);

    let stdout =
        String::from_utf8_lossy(
            &output.stdout
        )
        .into_owned();

    let stderr =
        String::from_utf8_lossy(
            &output.stderr
        )
        .into_owned();

    Ok(
        result_ok(
            process_result(
                status as i64,
                stdout,
                stderr,
            )
        )
    )
}


fn process_result(
    status: i64,
    stdout: String,
    stderr: String,
) -> Value {
    let mut object =
        Object::new();

    object.set_field(
        "status",
        Value::Int(status),
    );

    object.set_field(
        "stdout",
        Value::Str(
            Rc::new(stdout)
        ),
    );

    object.set_field(
        "stderr",
        Value::Str(
            Rc::new(stderr)
        ),
    );

    Value::Object(
        Rc::new(
            RefCell::new(object)
        )
    )
}

fn result_ok(value: Value) -> Value {
    // Use your existing Result constructor helper here
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Result",
                "Ok",
                vec![value],
            )
        )
    )
}

fn result_err(message: impl Into<String>) -> Value {
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Result",
                "Err",
                vec![
                    Value::Str(
                        Rc::new(
                            message.into()
                        )
                    )
                ],
            )
        )
    )
}

fn option_some(value: Value) -> Value {
    // Use your existing Option constructor helper here.
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "Some",
                vec![value],
            )
        )
    )
}

fn option_none() -> Value {
    // Use your existing Option constructor helper here.
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "None",
                vec![],
            )
        )
    )
}