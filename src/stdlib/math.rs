use crate::runtime::Value;

fn number(
    value: &Value,
) -> Result<f64, String> {
    match value {
        Value::Int(x) =>
            Ok(*x as f64),

        Value::Float(x) =>
            Ok(*x),

        other => Err(format!(
            "expected numeric value, got {}",
            other.type_name()
        )),
    }
}

pub fn sqrt(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "sqrt() expects exactly 1 argument"
                .into()
        );
    }

    let x =
        number(&args[0])?;

    Ok(Value::Float(x.sqrt()))
}

pub fn abs(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "abs() expects exactly 1 argument"
                .into()
        );
    }

    let x =
        number(&args[0])?;

    Ok(Value::Float(x.abs()))
}

pub fn exp(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "exp() expects exactly 1 argument"
                .into()
        );
    }

    let x =
        number(&args[0])?;

    Ok(Value::Float(x.exp()))
}

pub fn log(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "log() expects exactly 1 argument"
                .into()
        );
    }

    let x =
        number(&args[0])?;

    if x <= 0.0 {
        return Err(
            "log() requires a positive argument"
                .into()
        );
    }

    Ok(Value::Float(x.ln()))
}