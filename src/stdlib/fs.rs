use crate::runtime::{
    Module,
    ModuleRef,
    Value,
    EnumValue,
};

use std::{
    rc::Rc,
    cell::RefCell,
};

pub fn module() -> ModuleRef {
    let mut module =
        Module::new("fs");

    module.set_exported(
        "read",
        Value::Builtin(read),
    );

    module.set_exported(
        "write",
        Value::Builtin(write),
    );

    module.set_exported(
        "append",
        Value::Builtin(append),
    );

    module.set_exported(
        "exists",
        Value::Builtin(exists),
    );

    module.set_exported(
        "remove",
        Value::Builtin(remove),
    );

    module.set_exported(
        "mkdir",
        Value::Builtin(mkdir),
    );

    module.set_exported(
        "rename",
        Value::Builtin(rename),
    );

    module.set_exported(
        "copy",
        Value::Builtin(copy),
    );

    module.set_exported(
        "list_dir",
        Value::Builtin(list_dir),
    );

    Rc::new(
        RefCell::new(module)
    )
}


pub fn read(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "fs.read() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.read()",
        )?;

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            Ok(
                result_ok(
                    Value::Str(
                        Rc::new(text)
                    )
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to read '{}': {}",
                        path,
                        error
                    )
                )
            )
        }
    }
}

pub fn write(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "fs.write() expects exactly 2 arguments"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.write()",
        )?;

    let content =
        get_string(
            args.remove(0),
            "fs.write()",
            "second argument",
        )?;

    match std::fs::write(
        &path,
        content.as_bytes(),
    ) {
        Ok(()) => {
            Ok(
                result_ok(
                    Value::Unit
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to write '{}': {}",
                        path,
                        error
                    )
                )
            )
        }
    }
}

pub fn append(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "fs.append() expects exactly 2 arguments"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.append()",
        )?;

    let content =
        get_string(
            args.remove(0),
            "fs.append()",
            "second argument",
        )?;

    use std::io::Write;

    let result =
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| {
                file.write_all(
                    content.as_bytes()
                )
            });

    match result {
        Ok(()) => {
            Ok(
                result_ok(
                    Value::Unit
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to append to '{}': {}",
                        path,
                        error
                    )
                )
            )
        }
    }
}

pub fn exists(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "fs.exists() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.exists()",
        )?;

    Ok(
        Value::Bool(
            std::path::Path::new(&path)
                .exists()
        )
    )
}

pub fn remove(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "fs.remove() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.remove()",
        )?;

    match std::fs::remove_file(&path) {
        Ok(()) => {
            Ok(
                result_ok(
                    Value::Unit
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to remove '{}': {}",
                        path,
                        error
                    )
                )
            )
        }
    }
}

pub fn mkdir(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "fs.mkdir() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.mkdir()",
        )?;

    match std::fs::create_dir_all(&path) {
        Ok(()) => {
            Ok(
                result_ok(
                    Value::Unit
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to create directory '{}': {}",
                        path,
                        error
                    )
                )
            )
        }
    }
}

pub fn rename(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "fs.rename() expects exactly 2 arguments"
                .into()
        );
    }

    let from =
        get_path(
            &mut args,
            "fs.rename()",
        )?;

    let to =
        get_path(
            &mut args,
            "fs.rename()",
        )?;

    match std::fs::rename(
        &from,
        &to,
    ) {
        Ok(()) => {
            Ok(
                result_ok(
                    Value::Unit
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to rename '{}' to '{}': {}",
                        from,
                        to,
                        error
                    )
                )
            )
        }
    }
}

pub fn copy(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "fs.copy() expects exactly 2 arguments"
                .into()
        );
    }

    let from =
        get_path(
            &mut args,
            "fs.copy()",
        )?;

    let to =
        get_path(
            &mut args,
            "fs.copy()",
        )?;

    match std::fs::copy(
        &from,
        &to,
    ) {
        Ok(bytes) => {
            Ok(
                result_ok(
                    Value::Int(
                        bytes as i64
                    )
                )
            )
        }

        Err(error) => {
            Ok(
                result_err(
                    format!(
                        "failed to copy '{}' to '{}': {}",
                        from,
                        to,
                        error
                    )
                )
            )
        }
    }
}

pub fn list_dir(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "fs.list_dir() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        get_path(
            &mut args,
            "fs.list_dir()",
        )?;

    let entries =
        match std::fs::read_dir(&path) {
            Ok(entries) =>
                entries,

            Err(error) => {
                return Ok(
                    result_err(
                        format!(
                            "failed to read directory '{}': {}",
                            path,
                            error
                        )
                    )
                );
            }
        };

    let mut result =
        Vec::new();

    for entry in entries {
        let entry =
            match entry {
                Ok(entry) =>
                    entry,

                Err(error) => {
                    return Ok(
                        result_err(
                            format!(
                                "failed to read directory entry in '{}': {}",
                                path,
                                error
                            )
                        )
                    );
                }
            };

        result.push(
            Value::Str(
                Rc::new(
                    entry
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                )
            )
        );
    }

    Ok(
        result_ok(
            Value::List(
                Rc::new(
                    RefCell::new(
                        result
                    )
                )
            )
        )
    )
}

fn get_path(
    args: &mut Vec<Value>,
    function: &str,
) -> Result<String, String> {
    match args.remove(0) {
        Value::Str(path) =>
            Ok(path.as_ref().clone()),

        other =>
            Err(format!(
                "{} expects path as Str, got {}",
                function,
                other.type_name()
            )),
    }
}

fn get_string(
    value: Value,
    function: &str,
    position: &str,
) -> Result<String, String> {
    match value {
        Value::Str(value) =>
            Ok(value.as_ref().clone()),

        other =>
            Err(format!(
                "{} expects {} as Str, got {}",
                function,
                position,
                other.type_name()
            )),
    }
}

fn result_ok(value: Value) -> Value {
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