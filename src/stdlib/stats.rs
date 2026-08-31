use crate::runtime::{
    BuiltinFn, DataFrame, ExtensionRegistry, Module, ModuleRef, ReceiverKind, Series, SeriesRef,
    Value,
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use statrs::distribution::{ContinuousCDF, StudentsT};

struct FunctionSpec {
    name: &'static str,
    function: BuiltinFn,
    receiver: Option<ReceiverKind>,
}

fn function_specs() -> &'static [FunctionSpec] {
    &[
        FunctionSpec {
            name: "describe",
            function: describe,
            receiver: Some(ReceiverKind::DataFrame),
        },
        FunctionSpec {
            name: "sum",
            function: sum,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "min",
            function: min,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "max",
            function: max,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "mean",
            function: mean,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "median",
            function: median,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "quantile",
            function: quantile,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "variance",
            function: variance,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "std",
            function: std,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "correlation",
            function: correlation,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "ttest",
            function: ttest,
            receiver: None,
        },
        FunctionSpec {
            name: "welch",
            function: welch,
            receiver: Some(ReceiverKind::Series),
        },
    ]
}

pub fn module() -> ModuleRef {
    let mut module = Module::new("stats");

    for spec in function_specs() {
        module.set_exported(spec.name, Value::Builtin(spec.function));
    }

    Rc::new(RefCell::new(module))
}

pub fn register_extensions(registry: &mut ExtensionRegistry) {
    for spec in function_specs() {
        let Some(receiver) = spec.receiver else {
            continue;
        };

        registry.register(receiver, spec.name, Value::Builtin(spec.function));
    }
}

fn numeric_series(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) => series.numeric_values(),

        other => Err(format!(
            "stats function expects Series, got {}",
            other.type_name()
        )),
    }
}

fn numeric_values_from_series(series: &SeriesRef) -> Result<Vec<f64>, String> {
    series.numeric_values()
}

fn series_values(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) => series.numeric_values(),

        other => Err(format!(
            "stats function expects Series, got {}",
            other.type_name()
        )),
    }
}

fn result_dict(fields: Vec<(&str, Value)>) -> Value {
    let map = fields
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect::<HashMap<_, _>>();

    Value::Dict(Rc::new(RefCell::new(map)))
}

fn describe_series(
    series: &SeriesRef,
) -> Result<
    (
        usize,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ),
    String,
> {
    let values = numeric_values_from_series(series)?;

    if values.is_empty() {
        return Ok((0, None, None, None, None, None));
    }

    let count = values.len();

    let mean = values.iter().sum::<f64>() / count as f64;

    let variance = sample_variance(&values);

    let std = variance.map(|value| value.sqrt());

    let mut sorted = values.clone();

    sorted.sort_by(|a, b| a.total_cmp(b));

    let min = sorted[0];

    let median = quantile_sorted(&sorted, 0.5);

    let max = sorted[sorted.len() - 1];

    Ok((count, Some(mean), std, Some(min), Some(median), Some(max)))
}

pub fn describe(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("describe() expects exactly 1 argument".into());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),

        other => {
            return Err(format!(
                "describe() expects DataFrame, got {}",
                other.type_name()
            ));
        },
    };

    let columns = df.numeric_columns();

    if columns.is_empty() {
        return Err("describe() found no numeric columns".into());
    }

    let mut column_names = Vec::with_capacity(columns.len());

    let mut counts = Vec::with_capacity(columns.len());

    let mut means = Vec::with_capacity(columns.len());

    let mut stds = Vec::with_capacity(columns.len());

    let mut mins = Vec::with_capacity(columns.len());

    let mut medians = Vec::with_capacity(columns.len());

    let mut maxs = Vec::with_capacity(columns.len());

    for column in &columns {
        let (count, mean, std, min, median, max) = describe_series(column)?;

        column_names.push(Value::Str(Rc::new(column.name().to_owned())));

        counts.push(Value::Int(count as i64));

        means.push(mean.map(Value::Float).unwrap_or(Value::Null));

        stds.push(std.map(Value::Float).unwrap_or(Value::Null));

        mins.push(min.map(Value::Float).unwrap_or(Value::Null));

        medians.push(median.map(Value::Float).unwrap_or(Value::Null));

        maxs.push(max.map(Value::Float).unwrap_or(Value::Null));
    }

    DataFrame::from_series(vec![
        Rc::new(Series::new("column", column_names)),
        Rc::new(Series::new("count", counts)),
        Rc::new(Series::new("mean", means)),
        Rc::new(Series::new("std", stds)),
        Rc::new(Series::new("min", mins)),
        Rc::new(Series::new("median", medians)),
        Rc::new(Series::new("max", maxs)),
    ])
    .map(|df| Value::DataFrame(Rc::new(df)))
}

pub fn sum(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sum() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    Ok(Value::Float(values.iter().sum()))
}

pub fn min(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("min() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let result = values.iter().copied().reduce(f64::min).unwrap();

    Ok(Value::Float(result))
}

pub fn max(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("max() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let result = values.iter().copied().reduce(f64::max).unwrap();

    Ok(Value::Float(result))
}

pub fn mean(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("mean() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let sum = values.iter().sum::<f64>();

    Ok(Value::Float(sum / values.len() as f64))
}

fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }

    let position = q * (sorted.len() - 1) as f64;

    let lower = position.floor() as usize;

    let upper = position.ceil() as usize;

    if lower == upper {
        return sorted[lower];
    }

    let weight = position - lower as f64;

    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

pub fn median(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("median() expects exactly 1 argument".into());
    }

    let mut values = series_values(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    values.sort_by(|a, b| a.total_cmp(b));

    Ok(Value::Float(quantile_sorted(&values, 0.5)))
}

pub fn quantile(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("quantile() expects Series and q".into());
    }

    let values = series_values(&args[0])?;

    let q = match &args[1] {
        Value::Int(value) => *value as f64,

        Value::Float(value) => *value,

        other => {
            return Err(format!(
                "quantile() q must be numeric, got {}",
                other.type_name()
            ));
        },
    };

    if !q.is_finite() || !(0.0..=1.0).contains(&q) {
        return Err("quantile() q must be in [0, 1]".into());
    }

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let mut sorted = values;

    sorted.sort_by(|a, b| a.total_cmp(b));

    Ok(Value::Float(quantile_sorted(&sorted, q)))
}

fn sample_variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;

    let sum_squared = values
        .iter()
        .map(|value| {
            let diff = *value - mean;

            diff * diff
        })
        .sum::<f64>();

    Some(sum_squared / (values.len() - 1) as f64)
}

pub fn variance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("variance() expects exactly 1 argument".into());
    }

    let values = series_values(&args[0])?;

    let Some(variance) = sample_variance(&values) else {
        return Ok(Value::Null);
    };

    Ok(Value::Float(variance))
}

pub fn std(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("std() expects exactly 1 argument".into());
    }

    let values = series_values(&args[0])?;

    let Some(variance) = sample_variance(&values) else {
        return Ok(Value::Null);
    };

    Ok(Value::Float(variance.sqrt()))
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> Result<f64, String> {
    if x.len() != y.len() {
        return Err("correlation() requires equal-length Series".into());
    }

    if x.len() < 2 {
        return Err("correlation() requires at least 2 observations".into());
    }

    let mean_x = x.iter().sum::<f64>() / x.len() as f64;

    let mean_y = y.iter().sum::<f64>() / y.len() as f64;

    let mut covariance = 0.0;

    let mut variance_x = 0.0;

    let mut variance_y = 0.0;

    for (x_value, y_value) in x.iter().zip(y.iter()) {
        let dx = *x_value - mean_x;

        let dy = *y_value - mean_y;

        covariance += dx * dy;

        variance_x += dx * dx;

        variance_y += dy * dy;
    }

    if variance_x == 0.0 || variance_y == 0.0 {
        return Ok(f64::NAN);
    }

    Ok(covariance / (variance_x * variance_y).sqrt())
}

pub fn correlation(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("correlation() expects exactly 2 Series".into());
    }

    let x = series_values(&args[0])?;

    let y = series_values(&args[1])?;

    Ok(Value::Float(pearson_correlation(&x, &y)?))
}

pub fn ttest(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("ttest() expects Series and mu0".into());
    }

    let values = series_values(&args[0])?;

    let mu0 = match &args[1] {
        Value::Int(v) => *v as f64,

        Value::Float(v) => *v,

        other => {
            return Err(format!(
                "ttest() mu0 must be numeric, got {}",
                other.type_name()
            ));
        },
    };

    let n = values.len();

    if n < 2 {
        return Err("ttest() requires at least 2 observations".into());
    }

    let mean = values.iter().sum::<f64>() / n as f64;

    let variance = values
        .iter()
        .map(|x| {
            let d = *x - mean;

            d * d
        })
        .sum::<f64>()
        / (n - 1) as f64;

    let std = variance.sqrt();

    if std == 0.0 {
        return Err("ttest() sample standard deviation is zero".into());
    }

    let t = (mean - mu0) / (std / (n as f64).sqrt());

    let df = (n - 1) as f64;

    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    let p_value = 2.0 * distribution.sf(t.abs());

    Ok(result_dict(vec![
        ("statistic", Value::Float(t)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Float(df)),
        (
            "method",
            Value::Str(Rc::new("One-sample t-test".to_string())),
        ),
    ]))
}

pub fn welch(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("welch() expects exactly 2 Series".into());
    }

    let x = series_values(&args[0])?;

    let y = series_values(&args[1])?;

    if x.len() < 2 || y.len() < 2 {
        return Err("welch() requires at least 2 observations per group".into());
    }

    let nx = x.len() as f64;

    let ny = y.len() as f64;

    let mean_x = x.iter().sum::<f64>() / nx;

    let mean_y = y.iter().sum::<f64>() / ny;

    let var_x = x.iter().map(|v| (*v - mean_x).powi(2)).sum::<f64>() / (nx - 1.0);

    let var_y = y.iter().map(|v| (*v - mean_y).powi(2)).sum::<f64>() / (ny - 1.0);

    let se2 = var_x / nx + var_y / ny;

    if se2 == 0.0 {
        return Err("welch() standard error is zero".into());
    }

    let t = (mean_x - mean_y) / se2.sqrt();

    let numerator = se2.powi(2);

    let denominator =
        (var_x.powi(2) / (nx.powi(2) * (nx - 1.0))) + (var_y.powi(2) / (ny.powi(2) * (ny - 1.0)));

    let df = numerator / denominator;

    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    let p_value = 2.0 * distribution.sf(t.abs());

    Ok(result_dict(vec![
        ("statistic", Value::Float(t)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Float(df)),
        ("method", Value::Str(Rc::new("Welch's t-test".to_string()))),
    ]))
}
