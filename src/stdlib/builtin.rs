use crate::{
    runtime::{
        Value,
        IteratorObj,
        Env,
        Set,
    }
};

use std::{
    rc::Rc,
    cell::RefCell,
    io::{self, Write},
    time::Duration,
    collections::HashMap,
};

pub fn install_builtins(
    env: &Env
) {
    for (name, value) 
        in builtins() {
        env.define(name, value);
    }
}

fn builtins()
    -> HashMap<String, Value>
{
    let mut map = HashMap::new();

    map.insert(
        "print".into(),
        Value::Builtin(print),
    );

    map.insert(
        "typeof".into(),
        Value::Builtin(r#typeof),
    );

    map.insert(
        "iter".into(),
        Value::Builtin(iter),
    );

    map.insert(
        "set".into(),
        Value::Builtin(set),
    );

    map.insert(
        "range".into(),
        Value::Builtin(range),
    );

    map.insert(
        "len".into(),
        Value::Builtin(len),
    );

    map.insert(
        "is_null".into(),
        Value::Builtin(is_null),
    );

    map.insert(
        "is_type".into(),
        Value::Builtin(is_type),
    );

    map.insert(
        "assert".into(),
        Value::Builtin(assert),
    );

    map.insert(
        "panic".into(),
        Value::Builtin(panic),
    );

    map.insert(
        "input".into(),
        Value::Builtin(input),
    );

    map.insert(
        "str".into(),
        Value::Builtin(str),
    );

    map.insert(
        "int".into(),
        Value::Builtin(int),
    );

    map.insert(
        "float".into(),
        Value::Builtin(float),
    );

    map.insert(
        "bool".into(),
        Value::Builtin(bool),
    );

    map.insert(
        "args".into(),
        Value::Builtin(args),
    );

    map.insert(
        "env".into(),
        Value::Builtin(env),
    );

    map.insert(
        "cwd".into(),
        Value::Builtin(cwd),
    );

    map.insert(
        "sleep".into(),
        Value::Builtin(sleep),
    );

    map.insert(
        "random".into(),
        Value::Builtin(random),
    );

    map.insert(
        "randint".into(),
        Value::Builtin(randint),
    );

    map
}

fn expect_args(
    args: &[Value],
    expected: usize,
    name: &str,
) -> Result<(), String> {
    if args.len() != expected {
        return Err(format!(
            "{name}() expects exactly {expected} argument{}",
            if expected == 1 { "" } else { "s" }
        ));
    }

    Ok(())
}

fn expect_string(
    value: &Value,
    name: &str,
) -> Result<Rc<String>, String> {
    match value {
        Value::Str(s) => Ok(Rc::clone(s)),

        other => Err(format!(
            "{name}() expected Str, got {}",
            other.type_name()
        )),
    }
}

pub fn args(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(
            "args() expects no arguments".into()
        );
    }

    let values = std::env::args()
        .skip(1)
        .map(|arg| Value::Str(Rc::new(arg)))
        .collect();

    Ok(Value::List(
        Rc::new(std::cell::RefCell::new(values))
    ))
}

pub fn env(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "env() expects exactly 1 argument".into()
        );
    }

    let name = match &args[0] {
        Value::Str(name) => name,

        other => {
            return Err(format!(
                "env() expected Str, got {}",
                other.type_name()
            ));
        }
    };

    match std::env::var(name.as_ref()) {
        Ok(value) =>
            Ok(Value::Str(Rc::new(value))),

        Err(std::env::VarError::NotPresent) =>
            Ok(Value::Null),

        Err(e) =>
            Err(format!(
                "failed to read environment variable: {e}"
            )),
    }
}

pub fn cwd(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(
            "cwd() expects no arguments".into()
        );
    }

    let path = std::env::current_dir()
        .map_err(|e| e.to_string())?;

    Ok(Value::Str(
        Rc::new(
            path.to_string_lossy().into_owned()
        )
    ))
}

pub fn print(args: Vec<Value>) -> Result<Value,String> {
    for value in args { println!("{}", value); }
    Ok(Value::Unit)
}

pub fn input(args: Vec<Value>) -> Result<Value, String> {
    if args.len() > 1 {
        return Err(
            "input() expects zero or one argument".into()
        );
    }

    if let Some(prompt) = args.first() {
        print!("{}", prompt);
        io::stdout()
            .flush()
            .map_err(|e| e.to_string())?;
    }

    let mut line = String::new();

    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;

    if line.ends_with('\n') {
        line.pop();

        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(Value::Str(Rc::new(line)))
}

pub fn str(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "str() expects exactly 1 argument".into()
        );
    }

    Ok(Value::Str(
        Rc::new(args[0].to_string())
    ))
}

pub fn int(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "int() expects exactly 1 argument"
                .into()
        );
    }

    match args.into_iter().next().unwrap() {
        Value::Int(value) =>
            Ok(Value::Int(value)),

        Value::Float(value) => {
            if !value.is_finite() {
                return Err(
                    "cannot convert non-finite Float to Int"
                        .into()
                );
            }

            Ok(
                Value::Int(
                    value as i64
                )
            )
        }

        Value::Str(text) => {
            let value =
                text.trim()
                    .parse::<i64>()
                    .map_err(|error| {
                        format!(
                            "invalid integer '{}': {}",
                            text,
                            error
                        )
                    })?;

            Ok(
                Value::Int(value)
            )
        }

        other => {
            Err(
                format!(
                    "int() cannot convert {} to Int",
                    other.type_name()
                )
            )
        }
    }
}

pub fn float(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "float() expects exactly 1 argument"
                .into()
        );
    }

    match args.into_iter().next().unwrap() {
        Value::Float(value) =>
            Ok(Value::Float(value)),

        Value::Int(value) =>
            Ok(
                Value::Float(
                    value as f64
                )
            ),

        Value::Str(text) => {
            let value =
                text.trim()
                    .parse::<f64>()
                    .map_err(|error| {
                        format!(
                            "invalid float '{}': {}",
                            text,
                            error
                        )
                    })?;

            Ok(
                Value::Float(value)
            )
        }

        other => {
            Err(
                format!(
                    "float() cannot convert {} to Float",
                    other.type_name()
                )
            )
        }
    }
}

pub fn bool(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "bool() expects exactly 1 argument"
                .into()
        );
    }

    match args.into_iter().next().unwrap() {
        Value::Bool(value) =>
            Ok(Value::Bool(value)),

        Value::Str(text) => {
            match text.trim() {
                "true" =>
                    Ok(Value::Bool(true)),

                "false" =>
                    Ok(Value::Bool(false)),

                other =>
                    Err(
                        format!(
                            "invalid boolean '{}'",
                            other
                        )
                    ),
            }
        }

        other => {
            Err(
                format!(
                    "bool() cannot convert {} to Bool",
                    other.type_name()
                )
            )
        }
    }
}

pub fn r#typeof(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "typeof() expects exactly 1 argument".into()
        );
    }

    Ok(Value::Str(
        Rc::new(
            args[0].type_name().to_string()
        )
    ))
}

pub fn iter(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "iter() takes exactly 1 argument"
                .into()
        );
    }

    let value =
        args.remove(0);

    match value {
        Value::Iterator(iterator) => {
            Ok(
                Value::Iterator(iterator)
            )
        }

        Value::List(data) => {
            Ok(
                Value::Iterator(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data,
                                index: 0,
                            }
                        )
                    )
                )
            )
        }

        Value::Str(string) => {
            Ok(
                Value::Iterator(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Str {
                                data: Rc::new(
                                    string
                                        .chars()
                                        .collect()
                                ),
                                index: 0,
                            }
                        )
                    )
                )
            )
        }

        Value::Range(
            start,
            end,
            inclusive,
        ) => {
            let end =
                if inclusive {
                    end.checked_add(1)
                        .ok_or_else(|| {
                            "inclusive range endpoint overflow"
                                .to_owned()
                        })?
                } else {
                    end
                };

            Ok(
                Value::Iterator(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Range {
                                current: start,
                                end,
                            }
                        )
                    )
                )
            )
        }

        other => {
            Err(
                format!(
                    "{} is not iterable",
                    other.type_name()
                )
            )
        }
    }
}

pub fn set(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "set() expects exactly 1 argument"
                .into()
        );
    }

    let list =
        match &args[0] {
            Value::List(list) =>
                list.borrow(),

            other =>
                return Err(format!(
                    "set() expects List, got {}",
                    other.type_name()
                )),
        };

    let set =
        Set::from_values(
            list.clone()
        )?;

    Ok(
        Value::Set(
            Rc::new(
                RefCell::new(
                    set
                )
            )
        )
    )
}

pub fn range(args: Vec<Value>) -> Result<Value, String> {
    match args.len() {
        1 => {
            let end = match &args[0] {
                Value::Int(x) => *x,

                other => {
                    return Err(format!(
                        "range() expected Int, got {}",
                        other.type_name()
                    ));
                }
            };

            if end < 0 {
                return Err(
                    "range() requires a non-negative argument"
                        .into()
                );
            }

            let values = (0..end)
                .map(Value::Int)
                .collect::<Vec<_>>();

            Ok(Value::List(Rc::new(RefCell::new(values))))
        }

        2 => {
            let start = match &args[0] {
                Value::Int(x) => *x,

                other => {
                    return Err(format!(
                        "range() expected Int, got {}",
                        other.type_name()
                    ));
                }
            };

            let end = match &args[1] {
                Value::Int(x) => *x,

                other => {
                    return Err(format!(
                        "range() expected Int, got {}",
                        other.type_name()
                    ));
                }
            };

            let values = (start..end)
                .map(Value::Int)
                .collect::<Vec<_>>();

            Ok(Value::List(Rc::new(RefCell::new(values))))
        }

        _ => Err(
            "range() expects 1 or 2 arguments".into()
        ),
    }
}

pub fn len(args: Vec<Value>) -> Result<Value, String> {
    expect_args(&args, 1, "len")?;

    let length = match &args[0] {
        Value::List(data) => data.borrow().len(),

        Value::Str(s) => s.chars().count(),

        other => {
            return Err(format!(
                "len() does not support {}",
                other.type_name()
            ));
        }
    };

    Ok(Value::Int(length as i64))
}

pub fn random(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(
            "random() expects no arguments".into()
        );
    }

    Ok(Value::Float(
        rand::random::<f64>()
    ))
}

pub fn randint(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "randint() expects exactly 2 arguments".into()
        );
    }

    let min = match &args[0] {
        Value::Int(x) => *x,

        other => {
            return Err(format!(
                "randint() expected Int, got {}",
                other.type_name()
            ));
        }
    };

    let max = match &args[1] {
        Value::Int(x) => *x,

        other => {
            return Err(format!(
                "randint() expected Int, got {}",
                other.type_name()
            ));
        }
    };

    if min > max {
        return Err(
            "randint() requires min <= max".into()
        );
    }

    Ok(Value::Int(
        rand::random_range(min..=max)
    ))
}

pub fn is_null(args: Vec<Value>) -> Result<Value, String> {
    expect_args(&args, 1, "is_null")?;

    Ok(Value::Bool(
        matches!(args[0], Value::Null)
    ))
}

pub fn is_type(args: Vec<Value>) -> Result<Value, String> {
    expect_args(&args, 2, "is_type")?;

    let type_name = expect_string(&args[1], "is_type")?;

    Ok(Value::Bool(
        args[0].type_name() == type_name.as_str()
    ))
}

pub fn sleep(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "sleep() expects exactly 1 argument"
                .into()
        );
    }

    let ms = match &args[0] {
        Value::Int(ms) if *ms >= 0 => *ms,

        Value::Int(_) =>
            return Err(
                "sleep() requires a non-negative value"
                    .into()
            ),

        other =>
            return Err(format!(
                "sleep() expected Int, got {}",
                other.type_name()
            )),
    };

    std::thread::sleep(
        Duration::from_millis(ms as u64)
    );

    Ok(Value::Unit)
}

pub fn assert(args: Vec<Value>) -> Result<Value, String> {
    expect_args(&args, 1, "assert")?;

    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),

        Value::Bool(false) => Err(
            "assertion failed".into()
        ),

        other => Err(format!(
            "assert() expected Bool, got {}",
            other.type_name()
        )),
    }
}

pub fn panic(args: Vec<Value>) -> Result<Value, String> {
    if args.len() > 1 {
        return Err(
            "panic() expects zero or one argument"
                .into()
        );
    }

    let message = match args.first() {
        None =>
            "panic".to_string(),

        Some(Value::Str(message)) =>
            message.as_ref().clone(),

        Some(value) =>
            value.to_string(),
    };

    Err(message)
}

