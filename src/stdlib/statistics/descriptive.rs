use crate::runtime::Value;

fn collect_numeric_values<'a, I>(
    values: I,
) -> Result<Vec<f64>, String>
where
    I: IntoIterator<Item = &'a Value>,
{
    let mut result = Vec::new();

    for value in values {
        match value {
            Value::Int(v) =>
                result.push(*v as f64),

            Value::Float(v) =>
                result.push(*v),

            Value::Null => {
                // ignore
            }

            other => {
                return Err(format!(
                    "expected numeric value, got {}",
                    other.type_name()
                ));
            }
        }
    }

    if result.is_empty() {
        return Err(
            "no numeric observations".into()
        );
    }

    Ok(result)
}

pub(crate) fn collect_numbers(
    value: &Value,
) -> Result<Vec<f64>, String> {
    match value {
        Value::List(list) => {
            let list = list.borrow();

            collect_numeric_values(
                list.iter()
            )
        }

        Value::Series(series) => {
            collect_numeric_values(
                series.data().iter()
            )
        }

        other => {
            Err(format!(
                "expected List or Series, got {}",
                other.type_name()
            ))
        }
    }
}

pub fn sum(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "sum() expects exactly 1 argument".into()
        );
    }

    let values = collect_numbers(&args[0])?;

    Ok(Value::Float(
        values.iter().sum()
    ))
}

pub fn mean(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "mean() expects exactly 1 argument".into()
        );
    }

    let values = collect_numbers(&args[0])?;

    let mean =
        values.iter().sum::<f64>()
            / values.len() as f64;

    Ok(Value::Float(mean))
}

pub fn variance(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 && args.len() != 2 {
        return Err(
            "variance() expects 1 or 2 arguments".into()
        );
    }

    let values = collect_numbers(&args[0])?;

    let sample = if args.len() == 2 {
        match args[1] {
            Value::Bool(v) => v,

            _ => {
                return Err(
                    "variance() second argument must be Bool"
                        .into()
                );
            }
        }
    } else {
        false
    };

    if sample && values.len() < 2 {
        return Err(
            "sample variance requires at least 2 observations"
                .into()
        );
    }

    let mean =
        values.iter().sum::<f64>()
            / values.len() as f64;

    let ss =
        values
            .iter()
            .map(|x| {
                let d = *x - mean;
                d * d
            })
            .sum::<f64>();

    let denominator =
        if sample {
            (values.len() - 1) as f64
        } else {
            values.len() as f64
        };

    Ok(Value::Float(
        ss / denominator
    ))
}

pub fn std(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 && args.len() != 2 {
        return Err(
            "std() expects 1 or 2 arguments".into()
        );
    }

    let variance =
        match variance(args)? {
            Value::Float(v) => v,
            _ => unreachable!(),
        };

    Ok(Value::Float(variance.sqrt()))
}

pub fn median(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "median() expects exactly 1 argument".into()
        );
    }

    let mut values =
        collect_numbers(&args[0])?;

    values.sort_by(
        |a, b| a.total_cmp(b)
    );

    let n = values.len();

    let result =
        if n % 2 == 1 {
            values[n / 2]
        } else {
            (
                values[n / 2 - 1]
                + values[n / 2]
            ) / 2.0
        };

    Ok(Value::Float(result))
}

pub fn min(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "min() expects exactly 1 argument".into()
        );
    }

    let values =
        collect_numbers(&args[0])?;

    Ok(Value::Float(
        values
            .into_iter()
            .fold(f64::INFINITY, f64::min)
    ))
}

pub fn max(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "max() expects exactly 1 argument".into()
        );
    }

    let values =
        collect_numbers(&args[0])?;

    Ok(Value::Float(
        values
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
    ))
}

pub fn quantile(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "quantile() expects 2 arguments".into()
        );
    }

    let mut values =
        collect_numbers(&args[0])?;

    let q = match args[1] {
        Value::Int(v) => v as f64,
        Value::Float(v) => v,

        _ => {
            return Err(
                "quantile() probability must be numeric"
                    .into()
            );
        }
    };

    if !(0.0..=1.0).contains(&q) {
        return Err(
            "quantile() probability must be between 0 and 1"
                .into()
        );
    }

    values.sort_by(
        |a, b| a.total_cmp(b)
    );

    if values.len() == 1 {
        return Ok(Value::Float(values[0]));
    }

    let position =
        q * (values.len() - 1) as f64;

    let lower =
        position.floor() as usize;

    let upper =
        position.ceil() as usize;

    if lower == upper {
        return Ok(Value::Float(values[lower]));
    }

    let weight =
        position - lower as f64;

    let result =
        values[lower] * (1.0 - weight)
        + values[upper] * weight;

    Ok(Value::Float(result))
}

pub fn covariance(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "covariance() expects 2 arguments".into()
        );
    }

    let x = collect_numbers(&args[0])?;
    let y = collect_numbers(&args[1])?;

    if x.len() != y.len() {
        return Err(
            "covariance() requires equal sample sizes"
                .into()
        );
    }

    if x.len() < 2 {
        return Err(
            "covariance() requires at least 2 observations"
                .into()
        );
    }

    let n = x.len() as f64;

    let mean_x =
        x.iter().sum::<f64>() / n;

    let mean_y =
        y.iter().sum::<f64>() / n;

    let covariance =
        x.iter()
            .zip(y.iter())
            .map(|(x, y)| {
                (*x - mean_x) * (*y - mean_y)
            })
            .sum::<f64>()
            / (n - 1.0);

    Ok(Value::Float(covariance))
}

pub fn pearson(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "pearson() expects 2 arguments".into()
        );
    }

    let x = collect_numbers(&args[0])?;
    let y = collect_numbers(&args[1])?;

    if x.len() != y.len() {
        return Err(
            "pearson() requires equal sample sizes"
                .into()
        );
    }

    if x.len() < 2 {
        return Err(
            "pearson() requires at least 2 observations"
                .into()
        );
    }

    let n = x.len() as f64;

    let mean_x =
        x.iter().sum::<f64>() / n;

    let mean_y =
        y.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut sx = 0.0;
    let mut sy = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;

        numerator += dx * dy;
        sx += dx * dx;
        sy += dy * dy;
    }

    if sx == 0.0 || sy == 0.0 {
        return Err(
            "pearson() is undefined when either variable has zero variance"
                .into()
        );
    }

    Ok(Value::Float(
        numerator / (sx * sy).sqrt()
    ))
}

fn rank_values(
    values: &[f64],
) -> Vec<f64> {
    let mut indexed =
        values
            .iter()
            .enumerate()
            .map(|(i, v)| (i, *v))
            .collect::<Vec<_>>();

    indexed.sort_by(
        |a, b| a.1.total_cmp(&b.1)
    );

    let mut ranks =
        vec![0.0; values.len()];

    let mut i = 0;

    while i < indexed.len() {
        let mut j = i + 1;

        while j < indexed.len()
            && indexed[j].1 == indexed[i].1
        {
            j += 1;
        }

        let rank =
            (i + 1 + j) as f64 / 2.0;

        for k in i..j {
            ranks[indexed[k].0] = rank;
        }

        i = j;
    }

    ranks
}

pub fn spearman(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "spearman() expects 2 arguments".into()
        );
    }

    let x = collect_numbers(&args[0])?;
    let y = collect_numbers(&args[1])?;

    if x.len() != y.len() {
        return Err(
            "spearman() requires equal sample sizes"
                .into()
        );
    }

    let rx = rank_values(&x);
    let ry = rank_values(&y);

    pearson(vec![
        Value::List(std::rc::Rc::new(
            std::cell::RefCell::new(
                rx.into_iter()
                    .map(Value::Float)
                    .collect()
            )
        )),
        Value::List(std::rc::Rc::new(
            std::cell::RefCell::new(
                ry.into_iter()
                    .map(Value::Float)
                    .collect()
            )
        )),
    ])
}