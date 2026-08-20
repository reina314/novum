use crate::{
    runtime::{
        Value,
        IteratorObj,
    }
};

use std::{
    rc::Rc,
    cell::RefCell,
    io::{self, Write},
    time::Duration,
};

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

pub fn read(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "read() expects exactly 1 argument".into()
        );
    }

    let path = match &args[0] {
        Value::Str(path) => path,

        other => {
            return Err(format!(
                "read() expected Str, got {}",
                other.type_name()
            ));
        }
    };

    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| format!("failed to read '{path}': {e}"))?;

    Ok(Value::Str(Rc::new(text)))
}

pub fn write(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "write() expects exactly 2 arguments".into()
        );
    }

    let path = match &args[0] {
        Value::Str(path) => path,

        other => {
            return Err(format!(
                "write() expected path as Str, got {}",
                other.type_name()
            ));
        }
    };

    let content = args[1].to_string();

    std::fs::write(path.as_ref(), content)
        .map_err(|e| format!("failed to write '{path}': {e}"))?;

    Ok(Value::Unit)
}

pub fn append(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "append() expects exactly 2 arguments".into()
        );
    }

    let path = match &args[0] {
        Value::Str(path) => path,

        other => {
            return Err(format!(
                "append() expected path as Str, got {}",
                other.type_name()
            ));
        }
    };

    let content = args[1].to_string();

    use std::io::Write;

    let mut file =
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())
            .map_err(|e| {
                format!(
                    "failed to open '{path}': {e}"
                )
            })?;

    file.write_all(content.as_bytes())
        .map_err(|e| {
            format!(
                "failed to append to '{path}': {e}"
            )
        })?;

    Ok(Value::Unit)
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

pub fn int(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "parse_int() expects exactly 1 argument"
                .into()
        );
    }

    let text = match &args[0] {
        Value::Str(text) => text,

        other => {
            return Err(format!(
                "parse_int() expected Str, got {}",
                other.type_name()
            ));
        }
    };

    let value = text.parse::<i64>()
        .map_err(|e| format!("invalid integer: {e}"))?;

    Ok(Value::Int(value))
}

pub fn float(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "parse_float() expects exactly 1 argument"
                .into()
        );
    }

    let text = match &args[0] {
        Value::Str(text) => text,

        other => {
            return Err(format!(
                "parse_float() expected Str, got {}",
                other.type_name()
            ));
        }
    };

    let value = text.parse::<f64>()
        .map_err(|e| format!("invalid float: {e}"))?;

    Ok(Value::Float(value))
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

pub fn iter(mut args: Vec<Value>) -> Result<Value,String> {
    if args.len()!= 1 { 
        return Err(
            "iter() takes exactly 1 argument".into()
        ); 
    
    }
    match args.remove(0) {
        Value::Iterator(it) => 
            Ok(Value::Iterator(it)),
        
        Value::List(data) =>
            Ok(Value::Iterator(IteratorObj::List{data,index:0})),
        
        Value::Str(s) =>
            Ok(Value::Iterator(IteratorObj::Str{data:Rc::new(s.chars().collect()),index:0})),
        
        other =>
            Err(
                format!("{} is not iterable",other.type_name())
            ),
    }
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


