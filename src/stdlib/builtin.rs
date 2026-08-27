use crate::{
    runtime::{
        BuiltinFn,
        Value,
        IteratorObj,
        // Set,
        List,
        PathValue,
    }
};

use std::{
    rc::Rc,
    cell::RefCell,
    io::{self, Write},
    time::Duration,
};

const BUILTINS: &[(
    &str,
    BuiltinFn,
)] = &[
    ("print", print),
    ("typeof", r#typeof),
    ("iter", iter),
    // ("set", set),
    ("zip", zip),
    ("enumerate", enumerate),
    ("zeros", zeros),
    ("range", range),
    ("len", len),
    ("is_null", is_null),
    ("is_type", is_type),
    ("assert", assert),
    ("panic", panic),
    ("input", input),
    ("str", str),
    ("int", int),
    ("float", float),
    ("bool", bool),
    ("path", path),
    ("sleep", sleep),
    ("random", random),
    ("randint", randint),
];

pub fn get(
    name: &str,
) -> Option<Value> {
    BUILTINS
        .iter()
        .find(|(builtin_name, _)| {
            *builtin_name == name
        })
        .map(|(_, function)| {
            Value::Builtin(*function)
        })
}

pub fn contains(
    name: &str,
) -> bool {
    BUILTINS
        .iter()
        .any(|(builtin_name, _)| {
            *builtin_name == name
        })
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

    let value = args.into_iter().next().unwrap();

    let text = match value {
        Value::Path(path) =>
            path.to_string_lossy().to_owned(),

        value => value.to_string(),
    };

    Ok(Value::Str(Rc::new(text)))
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

pub fn path(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "path() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        match args.remove(0) {
            Value::Str(value) =>
                PathValue::new(
                    value.as_ref()
                ),

            Value::Path(value) =>
                value.as_ref().clone(),

            other =>
                return Err(
                    format!(
                        "path() expects Str or Path, got {}",
                        other.type_name()
                    )
                ),
        };

    Ok(
        Value::Path(
            Rc::new(path)
        )
    )
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

    let iterator = IteratorObj::from_value(value)?;

    Ok(
        Value::Iterator(
            iterator
        )
    )
}

// pub fn set(
//     args: Vec<Value>,
// ) -> Result<Value, String> {
//     if args.len() != 1 {
//         return Err(
//             "set() expects exactly 1 argument"
//                 .into()
//         );
//     }

//     let list =
//         match &args[0] {
//             Value::List(list) =>
//                 list.borrow(),

//             other =>
//                 return Err(format!(
//                     "set() expects List, got {}",
//                     other.type_name()
//                 )),
//         };

//     let set =
//         Set::from_values(
//             list.clone()
//         )?;

//     Ok(
//         Value::Set(
//             Rc::new(
//                 RefCell::new(
//                     set
//                 )
//             )
//         )
//     )
// }

pub fn zip(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "zip() expects exactly 2 arguments"
                .into()
        );
    }

    let left =
        IteratorObj::from_value(
            args.remove(0)
        )?;

    let right =
        IteratorObj::from_value(
            args.remove(0)
        )?;

    Ok(
        Value::Iterator(
            Rc::new(
                RefCell::new(
                    IteratorObj::Zip {
                        left,
                        right,
                    }
                )
            )
        )
    )
}

pub fn enumerate(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "enumerate() expects exactly 1 argument"
                .into()
        );
    }

    let source =
        IteratorObj::from_value(
            args.remove(0)
        )?;

    Ok(
        Value::Iterator(
            Rc::new(
                RefCell::new(
                    IteratorObj::Enumerate {
                        source,
                        index: 0,
                    }
                )
            )
        )
    )
}

pub fn zeros(
    mut args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "zeros() expects exactly 1 argument"
                .into()
        );
    }

    let count =
        match args.remove(0) {
            Value::Int(value)
                if value >= 0 =>
            {
                value as usize
            }

            Value::Int(_) =>
                return Err(
                    "zeros() does not accept a negative count"
                        .into()
                ),

            other =>
                return Err(format!(
                    "zeros() expects Int, got {}",
                    other.type_name()
                )),
        };

    Ok(
        Value::List(
            List::new(
                vec![
                    Value::Int(0);
                    count
                ]
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

            Ok(Value::List(List::new(values)))
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

            Ok(Value::List(List::new(values)))
        }

        _ => Err(
            "range() expects 1 or 2 arguments".into()
        ),
    }
}

pub fn len(args: Vec<Value>) -> Result<Value, String> {
    expect_args(&args, 1, "len")?;

    let length = match &args[0] {
        Value::List(data) => data.len(),

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

