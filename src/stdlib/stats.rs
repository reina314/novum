use crate::runtime::{
    BuiltinFn, DataFrame, ExtensionRegistry, Module, ModuleRef, ReceiverKind, Series, SeriesRef,
    Value,
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use statrs::distribution::{ContinuousCDF, Normal, StudentsT, FisherSnedecor, ChiSquared};

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
            name: "paired_ttest",
            function: paired_ttest,
            receiver: None,
        },
        FunctionSpec {
            name: "welch",
            function: welch,
            receiver: Some(ReceiverKind::Series),
        },
        FunctionSpec {
            name: "mann_whitney",
            function: mann_whitney,
            receiver: None,
        },
        FunctionSpec {
            name: "wilcoxon",
            function: wilcoxon,
            receiver: None,
        },
        FunctionSpec {
            name: "anova",
            function: anova,
            receiver: None,
        },
        FunctionSpec {
            name: "chi_square",
            function: chi_square,
            receiver: None,
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


//=========================
// general helpers
//=========================
fn numeric_series_values(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) => series.numeric_values(),

        other => Err(format!(
            "stats function expects Series, got {}",
            other.type_name()
        )),
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

fn expect_series_value(
    value: &Value,
    function: &str,
    index: usize,
) -> Result<SeriesRef, String> {
    match value {
        Value::Series(series) => {
            Ok(series.clone())
        }

        other => Err(format!(
            "{}() argument {} must be Series, got {}",
            function,
            index,
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

fn confidence_interval_dict(
    lower: f64,
    upper: f64,
    level: f64,
) -> Value {
    result_dict(vec![
        (
            "lower",
            Value::Float(lower),
        ),
        (
            "upper",
            Value::Float(upper),
        ),
        (
            "level",
            Value::Float(level),
        ),
    ])
}


//=========================
// descriptive statistics
//=========================
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

pub fn covariance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("covariance() expects exactly 2 Series".into());
    }

    let x = numeric_series_values(&args[0])?;

    let y = numeric_series_values(&args[1])?;

    Ok(Value::Float(covariance_value(&x, &y)?))
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


//=========================
// t-tests
//=========================
fn t_distribution_p_value(t: f64, df: f64) -> Result<f64, String> {
    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    Ok(2.0 * distribution.sf(t.abs()))
}

fn one_sample_ttest_values(
    values: &[f64],
    mu0: f64,
    method: &str,
    confidence: f64,
) -> Result<Value, String> {
    if !mu0.is_finite() {
        return Err(
            "t-test null hypothesis mean must be finite"
                .into()
        );
    }

    if values.len() < 2 {
        return Err(
            "t-test requires at least 2 observations"
                .into()
        );
    }

    if values.iter().any(
        |value| !value.is_finite()
    ) {
        return Err(
            "t-test data contains non-finite value"
                .into()
        );
    }

    let n =
        values.len();

    let n_f =
        n as f64;

    let mean =
        values.iter().sum::<f64>()
            / n_f;

    let variance =
        values
            .iter()
            .map(|value| {
                let difference =
                    *value - mean;

                difference * difference
            })
            .sum::<f64>()
            / (n - 1) as f64;

    if !variance.is_finite() {
        return Err(
            "t-test variance is not finite"
                .into()
        );
    }

    let std =
        variance.sqrt();

    if std == 0.0 {
        return Err(
            "t-test sample standard deviation is zero"
                .into()
        );
    }

    let estimate =
        mean - mu0;

    let standard_error =
        std / n_f.sqrt();

    let t =
        estimate / standard_error;

    let df =
        (n - 1) as f64;

    let p_value =
        t_distribution_p_value(
            t,
            df,
        )?;

    let (
        ci_lower,
        ci_upper,
    ) =
        t_confidence_interval(
            estimate,
            standard_error,
            df,
            confidence,
        )?;

    let effect_size =
        estimate / std;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(t),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "df",
            Value::Float(df),
        ),
        (
            "estimate",
            Value::Float(estimate),
        ),
        (
            "effect_size",
            Value::Float(effect_size),
        ),
        (
            "effect_size_name",
            Value::Str(
                Rc::new(
                    "Cohen's d"
                        .to_string()
                )
            ),
        ),
        (
            "confidence_interval",
            confidence_interval_dict(
                ci_lower,
                ci_upper,
                confidence,
            ),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    method.to_owned()
                )
            ),
        ),
    ]))
}

fn paired_numeric_values(
    first: &SeriesRef,
    second: &SeriesRef,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if first.len() != second.len() {
        return Err(
            "paired test requires equal-length Series"
                .into()
        );
    }

    let mut first_values =
        Vec::with_capacity(first.len());

    let mut second_values =
        Vec::with_capacity(second.len());

    for index in 0..first.len() {
        let first_value =
            first.get(index)
                .ok_or_else(|| {
                    format!(
                        "first Series index out of bounds: {}",
                        index
                    )
                })?;

        let second_value =
            second.get(index)
                .ok_or_else(|| {
                    format!(
                        "second Series index out of bounds: {}",
                        index
                    )
                })?;

        match (&first_value, &second_value) {
            (Value::Null, _)
            | (_, Value::Null) => {
                continue;
            }

            (
                Value::Int(first),
                Value::Int(second),
            ) => {
                first_values.push(
                    *first as f64
                );

                second_values.push(
                    *second as f64
                );
            }

            (
                Value::Int(first),
                Value::Float(second),
            ) => {
                if !second.is_finite() {
                    return Err(
                        "paired test contains non-finite value"
                            .into()
                    );
                }

                first_values.push(
                    *first as f64
                );

                second_values.push(
                    *second
                );
            }

            (
                Value::Float(first),
                Value::Int(second),
            ) => {
                if !first.is_finite() {
                    return Err(
                        "paired test contains non-finite value"
                            .into()
                    );
                }

                first_values.push(
                    *first
                );

                second_values.push(
                    *second as f64
                );
            }

            (
                Value::Float(first),
                Value::Float(second),
            ) => {
                if !first.is_finite()
                    || !second.is_finite()
                {
                    return Err(
                        "paired test contains non-finite value"
                            .into()
                    );
                }

                first_values.push(
                    *first
                );

                second_values.push(
                    *second
                );
            }

            (first, second) => {
                return Err(format!(
                    "paired test requires numeric Series; found {} and {}",
                    first.type_name(),
                    second.type_name()
                ));
            }
        }
    }

    Ok((
        first_values,
        second_values,
    ))
}

fn t_confidence_interval(
    estimate: f64,
    standard_error: f64,
    df: f64,
    confidence: f64,
) -> Result<(f64, f64), String> {
    if !estimate.is_finite()
        || !standard_error.is_finite()
        || standard_error < 0.0
        || !df.is_finite()
        || df <= 0.0
    {
        return Err(
            "invalid parameters for t confidence interval"
                .into()
        );
    }

    if !confidence.is_finite()
        || !(0.0 < confidence && confidence < 1.0)
    {
        return Err(
            "confidence level must be in (0, 1)"
                .into()
        );
    }

    let distribution =
        StudentsT::new(
            0.0,
            1.0,
            df,
        )
        .map_err(|error| error.to_string())?;

    let alpha =
        1.0 - confidence;

    let critical =
        distribution.inverse_cdf(
            1.0 - alpha / 2.0
        );

    let margin =
        critical * standard_error;

    Ok((
        estimate - margin,
        estimate + margin,
    ))
}

fn confidence_level_from_args(
    args: &[Value],
    index: usize,
    function: &str,
) -> Result<f64, String> {
    let value =
        match args.get(index) {
            Some(Value::Int(value)) => {
                *value as f64
            }

            Some(Value::Float(value)) => {
                *value
            }

            Some(other) => {
                return Err(format!(
                    "{}() confidence level must be numeric, got {}",
                    function,
                    other.type_name()
                ));
            }

            None => {
                return Ok(0.95);
            }
        };

    if !value.is_finite()
        || !(0.0 < value && value < 1.0)
    {
        return Err(format!(
            "{}() confidence level must be in (0, 1)",
            function
        ));
    }

    Ok(value)
}

fn paired_effect_size(
    differences: &[f64],
) -> Result<f64, String> {
    if differences.len() < 2 {
        return Err(
            "paired effect size requires at least 2 pairs"
                .into()
        );
    }

    let mean =
        differences.iter()
            .sum::<f64>()
            / differences.len() as f64;

    let variance =
        sample_variance(differences)
            .ok_or_else(|| {
                "paired effect size variance is undefined"
                    .to_string()
            })?;

    let std =
        variance.sqrt();

    if std == 0.0 {
        return Err(
            "paired effect size standard deviation is zero"
                .into()
        );
    }

    Ok(mean / std)
}

fn welch_effect_size(
    mean_x: f64,
    mean_y: f64,
    variance_x: f64,
    variance_y: f64,
) -> Result<f64, String> {
    let reference_sd =
        ((variance_x + variance_y) / 2.0)
            .sqrt();

    if !reference_sd.is_finite()
        || reference_sd == 0.0
    {
        return Err(
            "Welch effect size standard deviation is zero or non-finite"
                .into()
        );
    }

    Ok(
        (mean_x - mean_y)
            / reference_sd
    )
}

fn welch_confidence_interval(
    mean_x: f64,
    mean_y: f64,
    variance_x: f64,
    variance_y: f64,
    nx: usize,
    ny: usize,
    df: f64,
    confidence: f64,
) -> Result<(f64, f64), String> {
    let estimate =
        mean_x - mean_y;

    let standard_error =
        (
            variance_x / nx as f64
                + variance_y / ny as f64
        )
        .sqrt();

    t_confidence_interval(
        estimate,
        standard_error,
        df,
        confidence,
    )
}

pub fn ttest(
    args: Vec<Value>,
) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "ttest() expects Series, mu0, and optional confidence level"
                .into()
        );
    }

    let values =
        numeric_series_values(
            &args[0]
        )?;

    let mu0 =
        match &args[1] {
            Value::Int(value) => {
                *value as f64
            }

            Value::Float(value) => {
                *value
            }

            other => {
                return Err(format!(
                    "ttest() mu0 must be numeric, got {}",
                    other.type_name()
                ));
            }
        };

    let confidence =
        confidence_level_from_args(
            &args,
            2,
            "ttest",
        )?;

    one_sample_ttest_values(
        &values,
        mu0,
        "One-sample t-test",
        confidence,
    )
}

pub fn paired_ttest(
    args: Vec<Value>,
) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "paired_ttest() expects 2 Series and optional confidence level"
                .into()
        );
    }

    let first =
        expect_series_value(
            &args[0],
            "paired_ttest",
            0,
        )?;

    let second =
        expect_series_value(
            &args[1],
            "paired_ttest",
            1,
        )?;

    let (
        first_values,
        second_values,
    ) =
        paired_numeric_values(
            &first,
            &second,
        )?;

    if first_values.len() < 2 {
        return Err(
            "paired_ttest() requires at least 2 complete pairs"
                .into()
        );
    }

    let differences =
        first_values
            .iter()
            .zip(second_values.iter())
            .map(|(first, second)| {
                first - second
            })
            .collect::<Vec<_>>();

    let confidence =
        confidence_level_from_args(
            &args,
            2,
            "paired_ttest",
        )?;

    let result =
        one_sample_ttest_values(
            &differences,
            0.0,
            "Paired t-test",
            confidence,
        )?;

    /*
     * Replace the generic one-sample Cohen's d
     * with the paired-samples effect size d_z.
     */
    let effect_size =
        paired_effect_size(
            &differences
        )?;

    let Value::Dict(dict) =
        result
    else {
        unreachable!();
    };

    dict.borrow_mut().insert(
        "effect_size".into(),
        Value::Float(effect_size),
    );

    dict.borrow_mut().insert(
        "effect_size_name".into(),
        Value::Str(
            Rc::new(
                "Cohen's dz".to_string()
            )
        ),
    );

    Ok(Value::Dict(dict))
}

pub fn welch(
    args: Vec<Value>,
) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "welch() expects 2 Series and optional confidence level"
                .into()
        );
    }

    let x =
        numeric_series_values(
            &args[0]
        )?;

    let y =
        numeric_series_values(
            &args[1]
        )?;

    if x.len() < 2
        || y.len() < 2
    {
        return Err(
            "welch() requires at least 2 observations per group"
                .into()
        );
    }

    if x.iter().any(
        |value| !value.is_finite()
    )
        || y.iter().any(
            |value| !value.is_finite()
        )
    {
        return Err(
            "welch() data contains non-finite value"
                .into()
        );
    }

    let confidence =
        confidence_level_from_args(
            &args,
            2,
            "welch",
        )?;

    let nx =
        x.len() as f64;

    let ny =
        y.len() as f64;

    let mean_x =
        x.iter().sum::<f64>()
            / nx;

    let mean_y =
        y.iter().sum::<f64>()
            / ny;

    let var_x =
        x.iter()
            .map(|value| {
                (*value - mean_x).powi(2)
            })
            .sum::<f64>()
            / (nx - 1.0);

    let var_y =
        y.iter()
            .map(|value| {
                (*value - mean_y).powi(2)
            })
            .sum::<f64>()
            / (ny - 1.0);

    let se2 =
        var_x / nx
            + var_y / ny;

    if !se2.is_finite()
        || se2 <= 0.0
    {
        return Err(
            "welch() standard error is zero or non-finite"
                .into()
        );
    }

    let standard_error =
        se2.sqrt();

    let estimate =
        mean_x - mean_y;

    let t =
        estimate
            / standard_error;

    let numerator =
        se2.powi(2);

    let denominator =
        var_x.powi(2)
            / (
                nx.powi(2)
                    * (nx - 1.0)
            )
            + var_y.powi(2)
                / (
                    ny.powi(2)
                        * (ny - 1.0)
                );

    if denominator <= 0.0
        || !denominator.is_finite()
    {
        return Err(
            "welch() degrees of freedom are undefined"
                .into()
        );
    }

    let df =
        numerator / denominator;

    let p_value =
        t_distribution_p_value(
            t,
            df,
        )?;

    let (
        ci_lower,
        ci_upper,
    ) =
        welch_confidence_interval(
            mean_x,
            mean_y,
            var_x,
            var_y,
            x.len(),
            y.len(),
            df,
            confidence,
        )?;

    let effect_size =
        welch_effect_size(
            mean_x,
            mean_y,
            var_x,
            var_y,
        )?;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(t),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "df",
            Value::Float(df),
        ),
        (
            "estimate",
            Value::Float(estimate),
        ),
        (
            "effect_size",
            Value::Float(effect_size),
        ),
        (
            "effect_size_name",
            Value::Str(
                Rc::new(
                    "Standardized mean difference"
                        .to_string()
                )
            ),
        ),
        (
            "confidence_interval",
            confidence_interval_dict(
                ci_lower,
                ci_upper,
                confidence,
            ),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    "Welch's t-test"
                        .to_string()
                )
            ),
        ),
    ]))
}

//=========================
// non-parametric tests
//=========================
fn continuity_corrected_z(
    statistic: f64,
    mean: f64,
    standard_deviation: f64,
) -> Result<f64, String> {
    if !standard_deviation.is_finite()
        || standard_deviation <= 0.0
    {
        return Err(
            "normal approximation standard deviation must be positive and finite"
                .into()
        );
    }

    let correction =
        if statistic < mean {
            0.5
        } else if statistic > mean {
            -0.5
        } else {
            0.0
        };

    Ok(
        (statistic - mean + correction)
            / standard_deviation
    )
}

fn two_sided_normal_p_value(
    z: f64,
) -> Result<f64, String> {
    if !z.is_finite() {
        return Err(
            "normal approximation produced a non-finite z-score"
                .into()
        );
    }

    let normal =
        Normal::new(0.0, 1.0)
            .map_err(|error| error.to_string())?;

    Ok(
        2.0 * normal.sf(z.abs())
    )
}

fn rank_value(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn average_ranks(
    values: &[f64],
) -> Vec<f64> {
    let mut indexed =
        values
            .iter()
            .copied()
            .map(rank_value)
            .enumerate()
            .collect::<Vec<_>>();

    indexed.sort_by(
        |(_, left), (_, right)| {
            left.total_cmp(right)
        }
    );

    let mut ranks =
        vec![0.0; values.len()];

    let mut index = 0usize;

    while index < indexed.len() {
        let start = index;

        let value =
            indexed[index].1;

        while index < indexed.len()
            && indexed[index].1 == value
        {
            index += 1;
        }

        let end = index;

        let rank_start =
            start as f64 + 1.0;

        let rank_end =
            end as f64;

        let average =
            (rank_start + rank_end)
                / 2.0;

        for position in start..end {
            let original_index =
                indexed[position].0;

            ranks[original_index] =
                average;
        }
    }

    ranks
}

fn tie_group_sizes(
    values: &[f64],
) -> Vec<usize> {
    let mut sorted =
        values.to_vec();

    sorted.sort_by(
        |left, right| {
            left.total_cmp(right)
        }
    );

    let mut sizes =
        Vec::new();

    let mut index = 0usize;

    while index < sorted.len() {
        let start = index;

        let value =
            sorted[index];

        while index < sorted.len()
            && sorted[index] == value
        {
            index += 1;
        }

        sizes.push(index - start);
    }

    sizes
}

fn mann_whitney_values(
    x: &[f64],
    y: &[f64],
) -> Result<(f64, f64, f64), String> {
    if x.is_empty() || y.is_empty() {
        return Err(
            "mann_whitney() requires non-empty Series"
                .into()
        );
    }

    let nx = x.len();
    let ny = y.len();

    let mut combined =
        Vec::with_capacity(nx + ny);

    combined.extend(
        x.iter()
            .copied()
            .map(|value| (value, 0usize))
    );

    combined.extend(
        y.iter()
            .copied()
            .map(|value| (value, 1usize))
    );

    let values =
        combined
            .iter()
            .map(|(value, _)| *value)
            .collect::<Vec<_>>();

    let ranks =
        average_ranks(&values);

    let rank_sum_x =
        combined
            .iter()
            .enumerate()
            .filter(|(_, (_, group))| {
                *group == 0
            })
            .map(|(index, _)| ranks[index])
            .sum::<f64>();

    let u_x =
        rank_sum_x
            - (nx * (nx + 1) / 2) as f64;

    let u_y =
        (nx * ny) as f64
            - u_x;

    let u = u_x.min(u_y);

    let nx_f = nx as f64;
    let ny_f = ny as f64;
    let n = (nx + ny) as f64;

    let mean_u =
        nx_f * ny_f / 2.0;

    let tie_sizes =
        tie_group_sizes(&values);

    let tie_term =
        tie_sizes
            .iter()
            .map(|size| {
                let t = *size as f64;

                t.powi(3) - t
            })
            .sum::<f64>();

    let variance_u =
        nx_f * ny_f / 12.0
            * (
                n + 1.0
                - tie_term
                    / (n * (n - 1.0))
            );

    if variance_u <= 0.0
        || !variance_u.is_finite()
    {
        return Err(
            "mann_whitney() normal approximation is undefined"
                .into()
        );
    }

    let z =
        continuity_corrected_z(
            u,
            mean_u,
            variance_u.sqrt(),
        )?;

    let p_value =
        two_sided_normal_p_value(z)?;

    Ok((u, z, p_value))
}

pub fn mann_whitney(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "mann_whitney() expects exactly 2 Series"
                .into()
        );
    }

    let x =
        numeric_series_values(&args[0])?;

    let y =
        numeric_series_values(&args[1])?;

    let (statistic, z, p_value) =
        mann_whitney_values(&x, &y)?;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(statistic),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "z",
            Value::Float(z),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    "Mann-Whitney U test"
                        .to_string()
                )
            ),
        ),
    ]))
}

fn wilcoxon_variance(
    n: usize,
    tie_sizes: &[usize],
) -> f64 {
    let n = n as f64;

    let base =
        n * (n + 1.0) * (2.0 * n + 1.0)
            / 24.0;

    let tie_correction =
        tie_sizes
            .iter()
            .map(|size| {
                let t = *size as f64;

                t * (t + 1.0) * (2.0 * t + 1.0)
                    / 48.0
            })
            .sum::<f64>();

    base - tie_correction
}

fn wilcoxon_values(
    x: &[f64],
    y: &[f64],
) -> Result<(f64, f64, f64, usize), String> {
    if x.len() != y.len() {
        return Err(
            "wilcoxon() requires equal-length Series"
                .into()
        );
    }

    let mut differences =
        Vec::<f64>::new();

    for (x_value, y_value)
        in x.iter().zip(y.iter())
    {
        let difference =
            *x_value - *y_value;

        if difference != 0.0 {
            differences.push(
                difference
            );
        }
    }

    let n = differences.len();

    if n < 2 {
        return Err(
            "wilcoxon() requires at least 2 non-zero differences"
                .into()
        );
    }

    let absolute =
        differences
            .iter()
            .map(|value| value.abs())
            .collect::<Vec<_>>();

    let ranks =
        average_ranks(&absolute);

    let mut w_plus = 0.0;
    let mut w_minus = 0.0;

    for (difference, rank)
        in differences.iter().zip(ranks.iter())
    {
        if *difference > 0.0 {
            w_plus += rank;
        } else {
            w_minus += rank;
        }
    }

    let statistic =
        w_plus.min(w_minus);

    let n_f = n as f64;

    let mean =
        n_f * (n_f + 1.0) / 4.0;

    let tie_sizes =
        tie_group_sizes(&absolute);

    let variance =
        wilcoxon_variance(
            n,
            &tie_sizes,
        );

    if variance <= 0.0
        || !variance.is_finite()
    {
        return Err(
            "wilcoxon() normal approximation is undefined"
                .into()
        );
    }

    let z =
        continuity_corrected_z(
            statistic,
            mean,
            variance.sqrt(),
        )?;

    let p_value =
        two_sided_normal_p_value(z)?;

    Ok((
        statistic,
        z,
        p_value,
        n,
    ))
}

pub fn wilcoxon(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "wilcoxon() expects exactly 2 Series"
                .into()
        );
    }

    let x =
        numeric_series_values(&args[0])?;

    let y =
        numeric_series_values(&args[1])?;

    let (
        statistic,
        z,
        p_value,
        n,
    ) = wilcoxon_values(
        &x,
        &y,
    )?;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(statistic),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "z",
            Value::Float(z),
        ),
        (
            "n",
            Value::Int(n as i64),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    "Wilcoxon signed-rank test"
                        .to_string()
                )
            ),
        ),
    ]))
}

//=========================
// anova
//=========================
#[derive(Hash, Eq, PartialEq, Clone)]
enum CategoryKey {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
}

fn category_key(
    value: &Value,
) -> Result<Option<CategoryKey>, String> {
    match value {
        Value::Null => Ok(None),

        Value::Int(value) => {
            Ok(Some(CategoryKey::Int(*value)))
        }

        Value::Float(value) => {
            if !value.is_finite() {
                return Err(
                    "categorical factor contains non-finite Float"
                        .into()
                );
            }

            Ok(Some(
                CategoryKey::Float(
                    value.to_bits()
                )
            ))
        }

        Value::Bool(value) => {
            Ok(Some(CategoryKey::Bool(*value)))
        }

        Value::Str(value) => {
            Ok(Some(
                CategoryKey::Str(
                    value.as_ref().clone()
                )
            ))
        }

        other => {
            Err(format!(
                "unsupported categorical value: {}",
                other.type_name()
            ))
        }
    }
}

fn anova_groups(
    response: &SeriesRef,
    factor: &SeriesRef,
) -> Result<Vec<Vec<f64>>, String> {
    if response.len() != factor.len() {
        return Err(
            "anova() requires response and factor with equal lengths"
                .into()
        );
    }

    let mut groups =
        HashMap::<CategoryKey, Vec<f64>>::new();

    for index in 0..response.len() {
        let factor_value =
            factor.get(index)
                .ok_or_else(|| {
                    "factor index out of bounds".to_string()
                })?;

        let Some(key) =
            category_key(&factor_value)?
        else {
            continue;
        };

        let response_value =
            response.get(index)
                .ok_or_else(|| {
                    "response index out of bounds".to_string()
                })?;

        let value = match response_value {
            Value::Int(value) => {
                value as f64
            }

            Value::Float(value) => {
                if !value.is_finite() {
                    return Err(
                        "anova() response contains non-finite value"
                            .into()
                    );
                }

                value
            }

            Value::Null => {
                continue;
            }

            other => {
                return Err(format!(
                    "anova() response must be numeric, got {}",
                    other.type_name()
                ));
            }
        };

        groups
            .entry(key)
            .or_default()
            .push(value);
    }

    Ok(groups.into_values().collect())
}

fn one_way_anova_values(
    groups: &[Vec<f64>],
) -> Result<(f64, f64, f64, f64), String> {
    if groups.len() < 2 {
        return Err(
            "anova() requires at least 2 groups"
                .into()
        );
    }

    if groups.iter().any(Vec::is_empty) {
        return Err(
            "anova() groups must not be empty"
                .into()
        );
    }

    let total_n =
        groups
            .iter()
            .map(Vec::len)
            .sum::<usize>();

    if total_n <= groups.len() {
        return Err(
            "anova() requires positive within-group degrees of freedom"
                .into()
        );
    }

    let grand_sum =
        groups
            .iter()
            .flat_map(|group| group.iter())
            .sum::<f64>();

    let grand_mean =
        grand_sum / total_n as f64;

    let mut ss_between = 0.0;
    let mut ss_within = 0.0;

    for group in groups {
        let n =
            group.len() as f64;

        let mean =
            group.iter().sum::<f64>()
                / n;

        let mean_difference =
            mean - grand_mean;

        ss_between +=
            n * mean_difference
                * mean_difference;

        for value in group {
            let difference =
                *value - mean;

            ss_within +=
                difference * difference;
        }
    }

    let df_between =
        (groups.len() - 1) as f64;

    let df_within =
        (total_n - groups.len()) as f64;

    let ms_between =
        ss_between / df_between;

    let ms_within =
        ss_within / df_within;

    if ms_within == 0.0 {
        if ss_between == 0.0 {
            return Err(
                "anova() has zero within-group and between-group variance"
                    .into()
            );
        }

        return Ok((
            f64::INFINITY,
            df_between,
            df_within,
            0.0,
        ));
    }

    let f =
        ms_between / ms_within;

    let distribution =
        FisherSnedecor::new(
            df_between,
            df_within,
        )
        .map_err(|error| error.to_string())?;

    let p_value =
        distribution.sf(f);

    Ok((
        f,
        df_between,
        df_within,
        p_value,
    ))
}

pub fn anova(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(
            "anova() expects DataFrame, response column, and factor column"
                .into()
        );
    }

    let df = match &args[0] {
        Value::DataFrame(df) => {
            df.clone()
        }

        other => {
            return Err(format!(
                "anova() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        }
    };

    let response_name =
        match &args[1] {
            Value::Str(value) => {
                value.as_str()
            }

            other => {
                return Err(format!(
                    "anova() response column must be Str, got {}",
                    other.type_name()
                ));
            }
        };

    let factor_name =
        match &args[2] {
            Value::Str(value) => {
                value.as_str()
            }

            other => {
                return Err(format!(
                    "anova() factor column must be Str, got {}",
                    other.type_name()
                ));
            }
        };

    let response =
        df.column(response_name)
            .ok_or_else(|| {
                format!(
                    "anova() unknown response column '{}'",
                    response_name
                )
            })?;

    let factor =
        df.column(factor_name)
            .ok_or_else(|| {
                format!(
                    "anova() unknown factor column '{}'",
                    factor_name
                )
            })?;

    let groups =
        anova_groups(
            &response,
            &factor,
        )?;

    let (
        statistic,
        df_between,
        df_within,
        p_value,
    ) =
        one_way_anova_values(&groups)?;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(statistic),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "df_between",
            Value::Float(df_between),
        ),
        (
            "df_within",
            Value::Float(df_within),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    "One-way ANOVA".to_string()
                )
            ),
        ),
    ]))
}

//=========================
// chi-squared test
//=========================
fn chi_square_independence_values(
    table: &[Vec<usize>],
) -> Result<(f64, f64, usize), String> {
    let rows = table.len();

    if rows < 2 {
        return Err(
            "chi_square() requires at least 2 rows"
                .into()
        );
    }

    let columns =
        table[0].len();

    if columns < 2 {
        return Err(
            "chi_square() requires at least 2 columns"
                .into()
        );
    }

    if table.iter().any(
        |row| row.len() != columns
    ) {
        return Err(
            "chi_square() contingency table must be rectangular"
                .into()
        );
    }

    let mut row_totals =
        vec![0usize; rows];

    let mut column_totals =
        vec![0usize; columns];

    let mut total = 0usize;

    for row in 0..rows {
        for column in 0..columns {
            let value =
                table[row][column];

            row_totals[row] += value;
            column_totals[column] += value;
            total += value;
        }
    }

    if total == 0 {
        return Err(
            "chi_square() contingency table is empty"
                .into()
        );
    }

    let total_f =
        total as f64;

    let mut statistic = 0.0;

    for row in 0..rows {
        for column in 0..columns {
            let expected =
                row_totals[row] as f64
                    * column_totals[column] as f64
                    / total_f;

            if expected == 0.0 {
                continue;
            }

            let observed =
                table[row][column] as f64;

            let difference =
                observed - expected;

            statistic +=
                difference * difference
                    / expected;
        }
    }

    let df =
        (rows - 1)
            * (columns - 1);

    let distribution =
        ChiSquared::new(df as f64)
            .map_err(|error| {
                error.to_string()
            })?;

    let p_value =
        distribution.sf(statistic);

    Ok((
        statistic,
        p_value,
        df,
    ))
}

fn chi_square_table(
    first: &SeriesRef,
    second: &SeriesRef,
) -> Result<Vec<Vec<usize>>, String> {
    if first.len() != second.len() {
        return Err(
            "chi_square() requires equal-length columns"
                .into()
        );
    }

    let mut row_keys =
        Vec::<CategoryKey>::new();

    let mut column_keys =
        Vec::<CategoryKey>::new();

    let mut row_index =
        HashMap::<CategoryKey, usize>::new();

    let mut column_index =
        HashMap::<CategoryKey, usize>::new();

    let mut observations =
        Vec::<(usize, usize)>::new();

    for index in 0..first.len() {
        let first_value =
            first.get(index)
                .ok_or_else(|| {
                    "first column index out of bounds"
                        .to_string()
                })?;

        let second_value =
            second.get(index)
                .ok_or_else(|| {
                    "second column index out of bounds"
                        .to_string()
                })?;

        let Some(first_key) =
            category_key(&first_value)?
        else {
            continue;
        };

        let Some(second_key) =
            category_key(&second_value)?
        else {
            continue;
        };

        let first_position =
            if let Some(position) =
                row_index.get(&first_key)
            {
                *position
            } else {
                let position =
                    row_keys.len();

                row_index.insert(
                    first_key.clone(),
                    position,
                );

                row_keys.push(first_key);

                position
            };

        let second_position =
            if let Some(position) =
                column_index.get(&second_key)
            {
                *position
            } else {
                let position =
                    column_keys.len();

                column_index.insert(
                    second_key.clone(),
                    position,
                );

                column_keys.push(second_key);

                position
            };

        observations.push((
            first_position,
            second_position,
        ));
    }

    if row_keys.len() < 2
        || column_keys.len() < 2
    {
        return Err(
            "chi_square() requires at least 2 categories in each variable"
                .into()
        );
    }

    let mut table =
        vec![
            vec![0usize; column_keys.len()];
            row_keys.len()
        ];

    for (row, column)
        in observations
    {
        table[row][column] += 1;
    }

    Ok(table)
}

pub fn chi_square(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(
            "chi_square() expects DataFrame and two column names"
                .into()
        );
    }

    let df = match &args[0] {
        Value::DataFrame(df) => {
            df.clone()
        }

        other => {
            return Err(format!(
                "chi_square() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        }
    };

    let first_name =
        match &args[1] {
            Value::Str(value) => {
                value.as_str()
            }

            other => {
                return Err(format!(
                    "chi_square() first column name must be Str, got {}",
                    other.type_name()
                ));
            }
        };

    let second_name =
        match &args[2] {
            Value::Str(value) => {
                value.as_str()
            }

            other => {
                return Err(format!(
                    "chi_square() second column name must be Str, got {}",
                    other.type_name()
                ));
            }
        };

    let first =
        df.column(first_name)
            .ok_or_else(|| {
                format!(
                    "chi_square() unknown column '{}'",
                    first_name
                )
            })?;

    let second =
        df.column(second_name)
            .ok_or_else(|| {
                format!(
                    "chi_square() unknown column '{}'",
                    second_name
                )
            })?;

    let table =
        chi_square_table(
            &first,
            &second,
        )?;

    let (
        statistic,
        p_value,
        df_value,
    ) =
        chi_square_independence_values(
            &table
        )?;

    Ok(result_dict(vec![
        (
            "statistic",
            Value::Float(statistic),
        ),
        (
            "p_value",
            Value::Float(p_value),
        ),
        (
            "df",
            Value::Int(df_value as i64),
        ),
        (
            "method",
            Value::Str(
                Rc::new(
                    "Chi-square test of independence"
                        .to_string()
                )
            ),
        ),
    ]))
}
