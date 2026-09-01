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
            name: "range",
            function: range,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "skewness",
            function: skewness,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "kurtosis",
            function: kurtosis,
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
            name: "covariance",
            function: covariance,
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

fn numeric_series_values(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) => series.numeric_values(),

        other => Err(format!(
            "stats function expects Series, got {}",
            other.type_name()
        )),
    }
}

fn expect_series(args: &[Value], index: usize, function: &str) -> Result<SeriesRef, String> {
    match args.get(index) {
        Some(Value::Series(series)) => Ok(series.clone()),

        Some(other) => Err(format!(
            "{}() expects Series at argument {}, got {}",
            function,
            index,
            other.type_name()
        )),

        None => Err(format!("{}() missing argument {}", function, index)),
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

pub fn range(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("range() expects exactly 1 argument".into());
    }

    let values = numeric_series_values(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);

    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    Ok(Value::Float(max - min))
}

pub fn skewness(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("skewness() expects exactly 1 argument".into());
    }

    let values = numeric_series_values(&args[0])?;

    Ok(match sample_skewness(&values) {
        Some(value) => Value::Float(value),
        None => Value::Null,
    })
}

fn sample_skewness(values: &[f64]) -> Option<f64> {
    let n = values.len();

    if n < 3 {
        return None;
    }

    let mean = values.iter().sum::<f64>() / n as f64;

    let mut m2 = 0.0;
    let mut m3 = 0.0;

    for &value in values {
        let d = value - mean;

        m2 += d * d;
        m3 += d * d * d;
    }

    let s2 = m2 / (n - 1) as f64;

    if s2 == 0.0 {
        return Some(0.0);
    }

    let s = s2.sqrt();

    let g1 = (n as f64) / ((n - 1) as f64 * (n - 2) as f64) * (m3 / s.powi(3));

    Some(g1)
}

pub fn kurtosis(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("kurtosis() expects exactly 1 argument".into());
    }

    let values = numeric_series_values(&args[0])?;

    Ok(match sample_excess_kurtosis(&values) {
        Some(value) => Value::Float(value),
        None => Value::Null,
    })
}

fn sample_excess_kurtosis(values: &[f64]) -> Option<f64> {
    let n = values.len();

    if n < 4 {
        return None;
    }

    let mean = values.iter().sum::<f64>() / n as f64;

    let mut m2 = 0.0;
    let mut m4 = 0.0;

    for &value in values {
        let d = value - mean;

        let d2 = d * d;

        m2 += d2;
        m4 += d2 * d2;
    }

    if m2 == 0.0 {
        return Some(0.0);
    }

    let n = n as f64;

    let term1 = n * (n + 1.0) * m4 / ((n - 1.0) * (n - 2.0) * (n - 3.0) * m2.powi(2));

    let term2 = 3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0));

    Some(term1 - term2)
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

pub fn covariance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("covariance() expects exactly 2 Series".into());
    }

    let x = numeric_series_values(&args[0])?;

    let y = numeric_series_values(&args[1])?;

    Ok(Value::Float(covariance_value(&x, &y)?))
}

fn covariance_value(x: &[f64], y: &[f64]) -> Result<f64, String> {
    if x.len() != y.len() {
        return Err("covariance() requires equal-length Series".into());
    }

    if x.len() < 2 {
        return Err("covariance() requires at least 2 observations".into());
    }

    let mean_x = x.iter().sum::<f64>() / x.len() as f64;

    let mean_y = y.iter().sum::<f64>() / y.len() as f64;

    let covariance = x
        .iter()
        .zip(y.iter())
        .map(|(x, y)| (*x - mean_x) * (*y - mean_y))
        .sum::<f64>()
        / (x.len() - 1) as f64;

    Ok(covariance)
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> Result<f64, String> {
    if x.len() != y.len() {
        return Err("correlation() requires equal-length Series".into());
    }

    if x.len() < 2 {
        return Err("correlation() requires at least 2 observations".into());
    }

    let covariance = covariance_value(x, y)?;

    let mean_x = x.iter().sum::<f64>() / x.len() as f64;

    let mean_y = y.iter().sum::<f64>() / y.len() as f64;

    let variance_x = x
        .iter()
        .map(|value| {
            let diff = *value - mean_x;

            diff * diff
        })
        .sum::<f64>()
        / (x.len() - 1) as f64;

    let variance_y = y
        .iter()
        .map(|value| {
            let diff = *value - mean_y;

            diff * diff
        })
        .sum::<f64>()
        / (y.len() - 1) as f64;

    if variance_x == 0.0 || variance_y == 0.0 {
        return Ok(f64::NAN);
    }

    Ok(covariance / (variance_x * variance_y).sqrt())
}

pub fn correlation(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("correlation() expects exactly 2 Series".into());
    }

    let x = numeric_series_values(&args[0])?;

    let y = numeric_series_values(&args[1])?;

    Ok(Value::Float(pearson_correlation(&x, &y)?))
}

fn t_distribution_p_value(t: f64, df: f64) -> Result<f64, String> {
    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    Ok(2.0 * distribution.sf(t.abs()))
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

    let p_value = t_distribution_p_value(t, df)?;

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

    let p_value = t_distribution_p_value(t, df)?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(t)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Float(df)),
        ("method", Value::Str(Rc::new("Welch's t-test".to_string()))),
    ]))
}
