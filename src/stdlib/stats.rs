use crate::runtime::{
    BuiltinFn, DataFrame, ExtensionRegistry, Matrix, Module, ModuleRef, ReceiverKind, Series,
    SeriesRef, Value,
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use faer::{
    linalg::solvers::{DenseSolveCore, SolveLstsq},
    Mat,
};
use statrs::distribution::{ChiSquared, ContinuousCDF, FisherSnedecor, Normal, StudentsT};
use tukey_test::tukey_hsd;

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
            name: "cohens_d",
            function: cohens_d,
            receiver: None,
        },
        FunctionSpec {
            name: "hedges_g",
            function: hedges_g,
            receiver: None,
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
            name: "tukey",
            function: tukey,
            receiver: None,
        },
        FunctionSpec {
            name: "post_hoc",
            function: post_hoc,
            receiver: None,
        },
        FunctionSpec {
            name: "chi_square",
            function: chi_square,
            receiver: None,
        },
        FunctionSpec {
            name: "regression",
            function: regression,
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

// =========================
// general helpers
// =========================

fn numeric_series(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) => series.numeric_values(),

        other => Err(format!(
            "stats function expects Series, got {}",
            other.type_name()
        )),
    }
}

fn expect_series_value(value: &Value, function: &str, index: usize) -> Result<SeriesRef, String> {
    match value {
        Value::Series(series) => Ok(series.clone()),

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

fn confidence_interval_dict(lower: f64, upper: f64, level: f64) -> Value {
    result_dict(vec![
        ("lower", Value::Float(lower)),
        ("upper", Value::Float(upper)),
        ("level", Value::Float(level)),
    ])
}

fn confidence_level_from_args(args: &[Value], index: usize, function: &str) -> Result<f64, String> {
    let value = match args.get(index) {
        Some(Value::Int(value)) => *value as f64,

        Some(Value::Float(value)) => *value,

        Some(other) => {
            return Err(format!(
                "{}() confidence level must be numeric, got {}",
                function,
                other.type_name()
            ));
        },

        None => {
            return Ok(0.95);
        },
    };

    if !value.is_finite() || !(0.0 < value && value < 1.0) {
        return Err(format!("{}() confidence level must be in (0, 1)", function));
    }

    Ok(value)
}

fn validate_finite(values: &[f64], function: &str) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{}() data contains non-finite value", function));
    }

    Ok(())
}

fn numeric_argument(value: &Value, function: &str, name: &str) -> Result<f64, String> {
    let value = match value {
        Value::Int(value) => *value as f64,
        Value::Float(value) => *value,

        other => {
            return Err(format!(
                "{}() {} must be numeric, got {}",
                function,
                name,
                other.type_name()
            ));
        },
    };

    if !value.is_finite() {
        return Err(format!("{}() {} must be finite", function, name));
    }

    Ok(value)
}

fn expect_regression_predictors(value: &Value, function: &str) -> Result<Vec<SeriesRef>, String> {
    match value {
        Value::Series(series) => Ok(vec![series.clone()]),

        Value::List(list) => {
            let values = list.iter_cloned();

            if values.is_empty() {
                return Err(format!("{}() requires at least one predictor", function));
            }

            let mut predictors = Vec::with_capacity(values.len());

            for (index, value) in values.into_iter().enumerate() {
                match value {
                    Value::Series(series) => predictors.push(series),

                    other => {
                        return Err(format!(
                            "{}() predictor {} must be Series, got {}",
                            function,
                            index,
                            other.type_name()
                        ));
                    },
                }
            }

            Ok(predictors)
        },

        other => Err(format!(
            "{}() predictors must be Series or List of Series, got {}",
            function,
            other.type_name()
        )),
    }
}

// =========================
// descriptive statistics
// =========================

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
    let values = numeric_series(&Value::Series(series.clone()))?;

    if values.is_empty() {
        return Ok((0, None, None, None, None, None));
    }

    let count = values.len();
    let mean = values.iter().sum::<f64>() / count as f64;

    let variance = sample_variance(&values);
    let std = variance.map(f64::sqrt);

    let mut sorted = values;
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

    Ok(Value::Float(
        values.iter().copied().reduce(f64::min).unwrap(),
    ))
}

pub fn max(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("max() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    Ok(Value::Float(
        values.iter().copied().reduce(f64::max).unwrap(),
    ))
}

pub fn mean(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("mean() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    if values.is_empty() {
        return Ok(Value::Null);
    }

    Ok(Value::Float(
        values.iter().sum::<f64>() / values.len() as f64,
    ))
}

pub fn range(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("range() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

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
        let difference = value - mean;
        m2 += difference * difference;
        m3 += difference * difference * difference;
    }

    let s2 = m2 / (n - 1) as f64;

    if s2 == 0.0 {
        return Some(0.0);
    }

    let s = s2.sqrt();

    let g1 = n as f64 / ((n - 1) as f64 * (n - 2) as f64) * (m3 / s.powi(3));

    Some(g1)
}

pub fn skewness(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("skewness() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

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
        let difference = value - mean;
        let difference_squared = difference * difference;

        m2 += difference_squared;
        m4 += difference_squared * difference_squared;
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

    let values = numeric_series(&args[0])?;

    Ok(match sample_excess_kurtosis(&values) {
        Some(value) => Value::Float(value),
        None => Value::Null,
    })
}

pub fn median(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("median() expects exactly 1 argument".into());
    }

    let mut values = numeric_series(&args[0])?;

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

    let values = numeric_series(&args[0])?;
    let q = numeric_argument(&args[1], "quantile", "q")?;

    if !(0.0..=1.0).contains(&q) {
        return Err("quantile() q must be in [0, 1]".into());
    }

    if values.is_empty() {
        return Ok(Value::Null);
    }

    let mut sorted = values;
    sorted.sort_by(|a, b| a.total_cmp(b));

    Ok(Value::Float(quantile_sorted(&sorted, q)))
}

pub fn variance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("variance() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

    let Some(variance) = sample_variance(&values) else {
        return Ok(Value::Null);
    };

    Ok(Value::Float(variance))
}

pub fn std(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("std() expects exactly 1 argument".into());
    }

    let values = numeric_series(&args[0])?;

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
        .zip(y)
        .map(|(x, y)| (*x - mean_x) * (*y - mean_y))
        .sum::<f64>()
        / (x.len() - 1) as f64;

    Ok(covariance)
}

pub fn covariance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("covariance() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

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

    let variance_x = sample_variance(x).unwrap();
    let variance_y = sample_variance(y).unwrap();

    if variance_x == 0.0 || variance_y == 0.0 {
        return Ok(f64::NAN);
    }

    Ok(covariance / (variance_x * variance_y).sqrt())
}

pub fn correlation(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("correlation() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    Ok(Value::Float(pearson_correlation(&x, &y)?))
}

// =========================
// t-tests
// =========================

fn t_distribution_p_value(t: f64, df: f64) -> Result<f64, String> {
    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    Ok(2.0 * distribution.sf(t.abs()))
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
        return Err("invalid parameters for t confidence interval".into());
    }

    if !confidence.is_finite() || !(0.0 < confidence && confidence < 1.0) {
        return Err("confidence level must be in (0, 1)".into());
    }

    let distribution = StudentsT::new(0.0, 1.0, df).map_err(|error| error.to_string())?;

    let alpha = 1.0 - confidence;
    let critical = distribution.inverse_cdf(1.0 - alpha / 2.0);
    let margin = critical * standard_error;

    Ok((estimate - margin, estimate + margin))
}

fn one_sample_ttest_values(
    values: &[f64],
    mu0: f64,
    method: &str,
    confidence: f64,
) -> Result<Value, String> {
    if !mu0.is_finite() {
        return Err("t-test null hypothesis mean must be finite".into());
    }

    if values.len() < 2 {
        return Err("t-test requires at least 2 observations".into());
    }

    validate_finite(values, "t-test")?;

    let n = values.len();
    let n_f = n as f64;

    let mean = values.iter().sum::<f64>() / n_f;

    let variance = sample_variance(values).unwrap();

    if !variance.is_finite() {
        return Err("t-test variance is not finite".into());
    }

    let std = variance.sqrt();

    if std == 0.0 {
        return Err("t-test sample standard deviation is zero".into());
    }

    let estimate = mean - mu0;
    let standard_error = std / n_f.sqrt();
    let statistic = estimate / standard_error;
    let df = (n - 1) as f64;
    let p_value = t_distribution_p_value(statistic, df)?;

    let (ci_lower, ci_upper) = t_confidence_interval(estimate, standard_error, df, confidence)?;

    let effect_size = estimate / std;

    let effect_se = (1.0 / n_f + effect_size.powi(2) / (2.0 * n_f)).sqrt();
    let normal = Normal::new(0.0, 1.0).map_err(|error| error.to_string())?;
    let critical = normal.inverse_cdf(1.0 - (1.0 - confidence) / 2.0);
    let effect_margin = critical * effect_se;

    Ok(result_dict(vec![
        ("statistic", Value::Float(statistic)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Float(df)),
        ("estimate", Value::Float(estimate)),
        ("effect_size", Value::Float(effect_size)),
        (
            "effect_size_name",
            Value::Str(Rc::new("Cohen's d".to_string())),
        ),
        (
            "effect_size_ci",
            confidence_interval_dict(
                effect_size - effect_margin,
                effect_size + effect_margin,
                confidence,
            ),
        ),
        (
            "confidence_interval",
            confidence_interval_dict(ci_lower, ci_upper, confidence),
        ),
        ("method", Value::Str(Rc::new(method.to_owned()))),
    ]))
}

fn paired_numeric_values(
    first: &SeriesRef,
    second: &SeriesRef,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    if first.len() != second.len() {
        return Err("paired test requires equal-length Series".into());
    }

    let mut first_values = Vec::with_capacity(first.len());
    let mut second_values = Vec::with_capacity(second.len());

    for index in 0..first.len() {
        let first_value = first
            .get(index)
            .ok_or_else(|| format!("first Series index out of bounds: {}", index))?;

        let second_value = second
            .get(index)
            .ok_or_else(|| format!("second Series index out of bounds: {}", index))?;

        match (&first_value, &second_value) {
            (Value::Null, _) | (_, Value::Null) => continue,

            (Value::Int(first), Value::Int(second)) => {
                first_values.push(*first as f64);
                second_values.push(*second as f64);
            },

            (Value::Int(first), Value::Float(second)) => {
                if !second.is_finite() {
                    return Err("paired test contains non-finite value".into());
                }

                first_values.push(*first as f64);
                second_values.push(*second);
            },

            (Value::Float(first), Value::Int(second)) => {
                if !first.is_finite() {
                    return Err("paired test contains non-finite value".into());
                }

                first_values.push(*first);
                second_values.push(*second as f64);
            },

            (Value::Float(first), Value::Float(second)) => {
                if !first.is_finite() || !second.is_finite() {
                    return Err("paired test contains non-finite value".into());
                }

                first_values.push(*first);
                second_values.push(*second);
            },

            (first, second) => {
                return Err(format!(
                    "paired test requires numeric Series; found {} and {}",
                    first.type_name(),
                    second.type_name()
                ));
            },
        }
    }

    Ok((first_values, second_values))
}

fn paired_effect_size(differences: &[f64]) -> Result<f64, String> {
    if differences.len() < 2 {
        return Err("paired effect size requires at least 2 pairs".into());
    }

    let mean = differences.iter().sum::<f64>() / differences.len() as f64;

    let variance = sample_variance(differences)
        .ok_or_else(|| "paired effect size variance is undefined".to_string())?;

    let std = variance.sqrt();

    if std == 0.0 {
        return Err("paired effect size standard deviation is zero".into());
    }

    Ok(mean / std)
}

fn paired_effect_size_confidence_interval(
    effect_size: f64,
    n: usize,
    confidence: f64,
) -> Result<(f64, f64), String> {
    if n < 2 {
        return Err("paired effect size confidence interval requires at least 2 pairs".into());
    }

    let standard_error = (1.0 / n as f64 + effect_size.powi(2) / (2.0 * n as f64)).sqrt();

    let normal = Normal::new(0.0, 1.0).map_err(|error| error.to_string())?;
    let critical = normal.inverse_cdf(1.0 - (1.0 - confidence) / 2.0);

    let margin = critical * standard_error;

    Ok((effect_size - margin, effect_size + margin))
}

pub fn ttest(args: Vec<Value>) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err("ttest() expects Series, mu0, and optional confidence level".into());
    }

    let values = numeric_series(&args[0])?;
    let mu0 = numeric_argument(&args[1], "ttest", "mu0")?;
    let confidence = confidence_level_from_args(&args, 2, "ttest")?;

    one_sample_ttest_values(&values, mu0, "One-sample t-test", confidence)
}

pub fn paired_ttest(args: Vec<Value>) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err("paired_ttest() expects 2 Series and optional confidence level".into());
    }

    let first = expect_series_value(&args[0], "paired_ttest", 0)?;
    let second = expect_series_value(&args[1], "paired_ttest", 1)?;

    let (first_values, second_values) = paired_numeric_values(&first, &second)?;

    if first_values.len() < 2 {
        return Err("paired_ttest() requires at least 2 complete pairs".into());
    }

    let differences = first_values
        .iter()
        .zip(second_values.iter())
        .map(|(first, second)| first - second)
        .collect::<Vec<_>>();

    let confidence = confidence_level_from_args(&args, 2, "paired_ttest")?;

    let result = one_sample_ttest_values(&differences, 0.0, "Paired t-test", confidence)?;

    let effect_size = paired_effect_size(&differences)?;

    let (effect_lower, effect_upper) =
        paired_effect_size_confidence_interval(effect_size, differences.len(), confidence)?;

    let Value::Dict(dict) = result else {
        unreachable!();
    };

    dict.borrow_mut()
        .insert("effect_size".into(), Value::Float(effect_size));

    dict.borrow_mut().insert(
        "effect_size_name".into(),
        Value::Str(Rc::new("Cohen's dz".to_string())),
    );

    dict.borrow_mut().insert(
        "effect_size_ci".into(),
        confidence_interval_dict(effect_lower, effect_upper, confidence),
    );

    Ok(Value::Dict(dict))
}

fn cohens_d_independent(
    mean_x: f64,
    mean_y: f64,
    variance_x: f64,
    variance_y: f64,
    nx: usize,
    ny: usize,
) -> Result<f64, String> {
    if nx < 2 || ny < 2 {
        return Err("Cohen's d requires at least 2 observations per group".into());
    }

    if !mean_x.is_finite()
        || !mean_y.is_finite()
        || !variance_x.is_finite()
        || !variance_y.is_finite()
        || variance_x < 0.0
        || variance_y < 0.0
    {
        return Err("Cohen's d received invalid statistics".into());
    }

    let pooled_variance =
        (((nx - 1) as f64) * variance_x + ((ny - 1) as f64) * variance_y) / (nx + ny - 2) as f64;

    if pooled_variance <= 0.0 || !pooled_variance.is_finite() {
        return Err("Cohen's d pooled standard deviation is zero or non-finite".into());
    }

    Ok((mean_x - mean_y) / pooled_variance.sqrt())
}

fn hedges_g_from_d(d: f64, df: f64) -> Result<f64, String> {
    if !d.is_finite() || !df.is_finite() || df <= 0.0 {
        return Err("Hedges' g received invalid parameters".into());
    }

    let correction = 1.0 - 3.0 / (4.0 * df - 1.0);

    Ok(correction * d)
}

fn cohens_d_se(d: f64, nx: usize, ny: usize) -> Result<f64, String> {
    if nx < 2 || ny < 2 {
        return Err("Cohen's d standard error requires at least 2 observations per group".into());
    }

    if !d.is_finite() {
        return Err("Cohen's d is non-finite".into());
    }

    let total = (nx + ny) as f64;

    let sample_term = (nx + ny) as f64 / (nx as f64 * ny as f64);
    let effect_term = d.powi(2) / (2.0 * total);

    Ok((sample_term + effect_term).sqrt())
}

fn cohens_d_confidence_interval(d: f64, se: f64, confidence: f64) -> Result<(f64, f64), String> {
    if !d.is_finite() || !se.is_finite() || se < 0.0 {
        return Err("invalid Cohen's d confidence interval parameters".into());
    }

    let normal = Normal::new(0.0, 1.0).map_err(|error| error.to_string())?;
    let critical = normal.inverse_cdf(1.0 - (1.0 - confidence) / 2.0);
    let margin = critical * se;

    Ok((d - margin, d + margin))
}

fn standardized_mean_difference_result(
    d: f64,
    confidence: f64,
    nx: usize,
    ny: usize,
) -> Result<(Value, Value), String> {
    let se = cohens_d_se(d, nx, ny)?;
    let (lower, upper) = cohens_d_confidence_interval(d, se, confidence)?;

    let df = (nx + ny - 2) as f64;
    let g = hedges_g_from_d(d, df);

    let g = g?;

    let correction = if d == 0.0 { 1.0 } else { g / d };
    let g_se = se * correction.abs();

    let (g_lower, g_upper) = cohens_d_confidence_interval(g, g_se, confidence)?;

    Ok((
        confidence_interval_dict(lower, upper, confidence),
        confidence_interval_dict(g_lower, g_upper, confidence),
    ))
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
    let estimate = mean_x - mean_y;

    let standard_error = (variance_x / nx as f64 + variance_y / ny as f64).sqrt();

    t_confidence_interval(estimate, standard_error, df, confidence)
}

pub fn welch(args: Vec<Value>) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err("welch() expects 2 Series and optional confidence level".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    if x.len() < 2 || y.len() < 2 {
        return Err("welch() requires at least 2 observations per group".into());
    }

    validate_finite(&x, "welch")?;
    validate_finite(&y, "welch")?;

    let confidence = confidence_level_from_args(&args, 2, "welch")?;

    let nx = x.len() as f64;
    let ny = y.len() as f64;

    let mean_x = x.iter().sum::<f64>() / nx;
    let mean_y = y.iter().sum::<f64>() / ny;

    let var_x = x.iter().map(|value| (*value - mean_x).powi(2)).sum::<f64>() / (nx - 1.0);

    let var_y = y.iter().map(|value| (*value - mean_y).powi(2)).sum::<f64>() / (ny - 1.0);

    let se2 = var_x / nx + var_y / ny;

    if !se2.is_finite() || se2 <= 0.0 {
        return Err("welch() standard error is zero or non-finite".into());
    }

    let standard_error = se2.sqrt();
    let estimate = mean_x - mean_y;
    let statistic = estimate / standard_error;

    let numerator = se2.powi(2);

    let denominator =
        var_x.powi(2) / (nx.powi(2) * (nx - 1.0)) + var_y.powi(2) / (ny.powi(2) * (ny - 1.0));

    if denominator <= 0.0 || !denominator.is_finite() {
        return Err("welch() degrees of freedom are undefined".into());
    }

    let df = numerator / denominator;
    let p_value = t_distribution_p_value(statistic, df)?;

    let (ci_lower, ci_upper) = welch_confidence_interval(
        mean_x,
        mean_y,
        var_x,
        var_y,
        x.len(),
        y.len(),
        df,
        confidence,
    )?;

    let effect_size = cohens_d_independent(mean_x, mean_y, var_x, var_y, x.len(), y.len())?;

    let (d_confidence_interval, g_confidence_interval) =
        standardized_mean_difference_result(effect_size, confidence, x.len(), y.len())?;

    let hedges_g = hedges_g_from_d(effect_size, (x.len() + y.len() - 2) as f64)?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(statistic)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Float(df)),
        ("estimate", Value::Float(estimate)),
        ("effect_size", Value::Float(effect_size)),
        (
            "effect_size_name",
            Value::Str(Rc::new("Cohen's d".to_string())),
        ),
        ("effect_size_ci", d_confidence_interval),
        ("hedges_g", Value::Float(hedges_g)),
        ("hedges_g_ci", g_confidence_interval),
        (
            "confidence_interval",
            confidence_interval_dict(ci_lower, ci_upper, confidence),
        ),
        ("method", Value::Str(Rc::new("Welch's t-test".to_string()))),
    ]))
}

// =========================
// effect sizes
// =========================

pub fn cohens_d(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("cohens_d() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    if x.len() < 2 || y.len() < 2 {
        return Err("cohens_d() requires at least 2 observations per group".into());
    }

    validate_finite(&x, "cohens_d")?;
    validate_finite(&y, "cohens_d")?;

    let nx = x.len() as f64;
    let ny = y.len() as f64;

    let mean_x = x.iter().sum::<f64>() / nx;
    let mean_y = y.iter().sum::<f64>() / ny;

    let variance_x = sample_variance(&x).unwrap();
    let variance_y = sample_variance(&y).unwrap();

    let d = cohens_d_independent(mean_x, mean_y, variance_x, variance_y, x.len(), y.len())?;

    Ok(Value::Float(d))
}

pub fn hedges_g(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("hedges_g() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    if x.len() < 2 || y.len() < 2 {
        return Err("hedges_g() requires at least 2 observations per group".into());
    }

    validate_finite(&x, "hedges_g")?;
    validate_finite(&y, "hedges_g")?;

    let nx = x.len() as f64;
    let ny = y.len() as f64;

    let mean_x = x.iter().sum::<f64>() / nx;
    let mean_y = y.iter().sum::<f64>() / ny;

    let variance_x = sample_variance(&x).unwrap();
    let variance_y = sample_variance(&y).unwrap();

    let d = cohens_d_independent(mean_x, mean_y, variance_x, variance_y, x.len(), y.len())?;

    hedges_g_from_d(d, (x.len() + y.len() - 2) as f64).map(Value::Float)
}

// =========================
// non-parametric tests
// =========================

fn continuity_corrected_z(
    statistic: f64,
    mean: f64,
    standard_deviation: f64,
) -> Result<f64, String> {
    if !standard_deviation.is_finite() || standard_deviation <= 0.0 {
        return Err("normal approximation standard deviation must be positive and finite".into());
    }

    let correction = if statistic < mean {
        0.5
    } else if statistic > mean {
        -0.5
    } else {
        0.0
    };

    Ok((statistic - mean + correction) / standard_deviation)
}

fn two_sided_normal_p_value(z: f64) -> Result<f64, String> {
    if !z.is_finite() {
        return Err("normal approximation produced a non-finite z-score".into());
    }

    let normal = Normal::new(0.0, 1.0).map_err(|error| error.to_string())?;

    Ok(2.0 * normal.sf(z.abs()))
}

fn rank_value(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut indexed = values
        .iter()
        .copied()
        .map(rank_value)
        .enumerate()
        .collect::<Vec<_>>();

    indexed.sort_by(|(_, left), (_, right)| left.total_cmp(right));

    let mut ranks = vec![0.0; values.len()];
    let mut index = 0usize;

    while index < indexed.len() {
        let start = index;
        let value = indexed[index].1;

        while index < indexed.len() && indexed[index].1 == value {
            index += 1;
        }

        let end = index;
        let average = (start as f64 + 1.0 + end as f64) / 2.0;

        for position in start..end {
            ranks[indexed[position].0] = average;
        }
    }

    ranks
}

fn tie_group_sizes(values: &[f64]) -> Vec<usize> {
    let mut sorted = values.to_vec();

    sorted.sort_by(|left, right| left.total_cmp(right));

    let mut sizes = Vec::new();
    let mut index = 0usize;

    while index < sorted.len() {
        let start = index;
        let value = sorted[index];

        while index < sorted.len() && sorted[index] == value {
            index += 1;
        }

        sizes.push(index - start);
    }

    sizes
}

fn mann_whitney_values(x: &[f64], y: &[f64]) -> Result<(f64, f64, f64), String> {
    if x.is_empty() || y.is_empty() {
        return Err("mann_whitney() requires non-empty Series".into());
    }

    validate_finite(x, "mann_whitney")?;
    validate_finite(y, "mann_whitney")?;

    let nx = x.len();
    let ny = y.len();

    let mut combined = Vec::with_capacity(nx + ny);

    combined.extend(x.iter().copied().map(|value| (value, 0usize)));
    combined.extend(y.iter().copied().map(|value| (value, 1usize)));

    let values = combined.iter().map(|(value, _)| *value).collect::<Vec<_>>();

    let ranks = average_ranks(&values);

    let rank_sum_x = combined
        .iter()
        .enumerate()
        .filter(|(_, (_, group))| *group == 0)
        .map(|(index, _)| ranks[index])
        .sum::<f64>();

    let u_x = rank_sum_x - (nx * (nx + 1) / 2) as f64;
    let u_y = (nx * ny) as f64 - u_x;
    let u = u_x.min(u_y);

    let nx_f = nx as f64;
    let ny_f = ny as f64;
    let n = (nx + ny) as f64;

    let mean_u = nx_f * ny_f / 2.0;

    let tie_term = tie_group_sizes(&values)
        .iter()
        .map(|size| {
            let t = *size as f64;
            t.powi(3) - t
        })
        .sum::<f64>();

    let variance_u = nx_f * ny_f / 12.0 * (n + 1.0 - tie_term / (n * (n - 1.0)));

    if variance_u <= 0.0 || !variance_u.is_finite() {
        return Err("mann_whitney() normal approximation is undefined".into());
    }

    let z = continuity_corrected_z(u, mean_u, variance_u.sqrt())?;
    let p_value = two_sided_normal_p_value(z)?;

    Ok((u, z, p_value))
}

pub fn mann_whitney(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("mann_whitney() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    let (statistic, z, p_value) = mann_whitney_values(&x, &y)?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(statistic)),
        ("p_value", Value::Float(p_value)),
        ("z", Value::Float(z)),
        (
            "method",
            Value::Str(Rc::new("Mann-Whitney U test".to_string())),
        ),
    ]))
}

fn wilcoxon_variance(n: usize, tie_sizes: &[usize]) -> f64 {
    let n = n as f64;

    let base = n * (n + 1.0) * (2.0 * n + 1.0) / 24.0;

    let tie_correction = tie_sizes
        .iter()
        .map(|size| {
            let t = *size as f64;
            t * (t + 1.0) * (2.0 * t + 1.0) / 48.0
        })
        .sum::<f64>();

    base - tie_correction
}

fn wilcoxon_values(x: &[f64], y: &[f64]) -> Result<(f64, f64, f64, usize), String> {
    if x.len() != y.len() {
        return Err("wilcoxon() requires equal-length Series".into());
    }

    validate_finite(x, "wilcoxon")?;
    validate_finite(y, "wilcoxon")?;

    let differences = x
        .iter()
        .zip(y)
        .map(|(x, y)| *x - *y)
        .filter(|difference| *difference != 0.0)
        .collect::<Vec<_>>();

    let n = differences.len();

    if n < 2 {
        return Err("wilcoxon() requires at least 2 non-zero differences".into());
    }

    let absolute = differences
        .iter()
        .map(|value| value.abs())
        .collect::<Vec<_>>();

    let ranks = average_ranks(&absolute);

    let mut w_plus = 0.0;
    let mut w_minus = 0.0;

    for (difference, rank) in differences.iter().zip(ranks.iter()) {
        if *difference > 0.0 {
            w_plus += rank;
        } else {
            w_minus += rank;
        }
    }

    let statistic = w_plus.min(w_minus);

    let n_f = n as f64;
    let mean = n_f * (n_f + 1.0) / 4.0;

    let variance = wilcoxon_variance(n, &tie_group_sizes(&absolute));

    if variance <= 0.0 || !variance.is_finite() {
        return Err("wilcoxon() normal approximation is undefined".into());
    }

    let z = continuity_corrected_z(statistic, mean, variance.sqrt())?;
    let p_value = two_sided_normal_p_value(z)?;

    Ok((statistic, z, p_value, n))
}

pub fn wilcoxon(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("wilcoxon() expects exactly 2 Series".into());
    }

    let x = numeric_series(&args[0])?;
    let y = numeric_series(&args[1])?;

    let (statistic, z, p_value, n) = wilcoxon_values(&x, &y)?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(statistic)),
        ("p_value", Value::Float(p_value)),
        ("z", Value::Float(z)),
        ("n", Value::Int(n as i64)),
        (
            "method",
            Value::Str(Rc::new("Wilcoxon signed-rank test".to_string())),
        ),
    ]))
}

// =========================
// ANOVA
// =========================

#[derive(Hash, Eq, PartialEq, Clone)]
enum CategoryKey {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
}

fn category_key(value: &Value) -> Result<Option<CategoryKey>, String> {
    match value {
        Value::Null => Ok(None),

        Value::Int(value) => Ok(Some(CategoryKey::Int(*value))),

        Value::Float(value) => {
            if !value.is_finite() {
                return Err("categorical factor contains non-finite Float".into());
            }

            let canonical = if *value == 0.0 { 0.0 } else { *value };

            Ok(Some(CategoryKey::Float(canonical.to_bits())))
        },

        Value::Bool(value) => Ok(Some(CategoryKey::Bool(*value))),

        Value::Str(value) => Ok(Some(CategoryKey::Str(value.as_ref().clone()))),

        other => Err(format!(
            "unsupported categorical value: {}",
            other.type_name()
        )),
    }
}

struct AnovaGroup {
    key: CategoryKey,
    label: Value,
    values: Vec<f64>,
}

struct AnovaSummary {
    statistic: f64,
    p_value: f64,
    df_between: f64,
    df_within: f64,
    ss_between: f64,
    ss_within: f64,
    ss_total: f64,
}

fn collect_anova_groups(
    response: &SeriesRef,
    factor: &SeriesRef,
) -> Result<Vec<AnovaGroup>, String> {
    if response.len() != factor.len() {
        return Err("anova() requires response and factor with equal lengths".into());
    }

    let mut groups = Vec::<AnovaGroup>::new();
    let mut positions = HashMap::<CategoryKey, usize>::new();

    for index in 0..response.len() {
        let factor_value = factor
            .get(index)
            .ok_or_else(|| format!("factor index out of bounds: {}", index))?;

        let Some(key) = category_key(&factor_value)? else {
            continue;
        };

        let response_value = response
            .get(index)
            .ok_or_else(|| format!("response index out of bounds: {}", index))?;

        let value = match response_value {
            Value::Int(value) => value as f64,

            Value::Float(value) => {
                if !value.is_finite() {
                    return Err("anova() response contains non-finite value".into());
                }

                value
            },

            Value::Null => continue,

            other => {
                return Err(format!(
                    "anova() response must be numeric, got {}",
                    other.type_name()
                ));
            },
        };

        if let Some(&group_index) = positions.get(&key) {
            groups[group_index].values.push(value);
        } else {
            let group_index = groups.len();

            positions.insert(key.clone(), group_index);

            groups.push(AnovaGroup {
                key,
                label: factor_value,
                values: vec![value],
            });
        }
    }

    Ok(groups)
}

fn anova_groups(response: &SeriesRef, factor: &SeriesRef) -> Result<Vec<Vec<f64>>, String> {
    Ok(collect_anova_groups(response, factor)?
        .into_iter()
        .map(|group| group.values)
        .collect())
}

fn one_way_anova_values(groups: &[Vec<f64>]) -> Result<AnovaSummary, String> {
    if groups.len() < 2 {
        return Err("anova() requires at least 2 groups".into());
    }

    if groups.iter().any(Vec::is_empty) {
        return Err("anova() groups must not be empty".into());
    }

    let total_n = groups.iter().map(Vec::len).sum::<usize>();

    if total_n <= groups.len() {
        return Err("anova() requires positive within-group degrees of freedom".into());
    }

    let grand_sum = groups.iter().flat_map(|group| group.iter()).sum::<f64>();
    let grand_mean = grand_sum / total_n as f64;

    let mut ss_between = 0.0;
    let mut ss_within = 0.0;

    for group in groups {
        let n = group.len() as f64;
        let mean = group.iter().sum::<f64>() / n;

        let difference = mean - grand_mean;
        ss_between += n * difference * difference;

        for value in group {
            let difference = *value - mean;
            ss_within += difference * difference;
        }
    }

    let ss_total = ss_between + ss_within;
    let df_between = (groups.len() - 1) as f64;
    let df_within = (total_n - groups.len()) as f64;

    let ms_between = ss_between / df_between;
    let ms_within = ss_within / df_within;

    let statistic = if ms_within == 0.0 {
        if ms_between == 0.0 {
            return Err("anova() has zero between-group and within-group variance".into());
        }

        f64::INFINITY
    } else {
        ms_between / ms_within
    };

    let p_value = if statistic.is_infinite() {
        0.0
    } else {
        let distribution =
            FisherSnedecor::new(df_between, df_within).map_err(|error| error.to_string())?;

        distribution.sf(statistic)
    };

    Ok(AnovaSummary {
        statistic,
        p_value,
        df_between,
        df_within,
        ss_between,
        ss_within,
        ss_total,
    })
}

fn eta_squared(ss_between: f64, ss_total: f64) -> Result<f64, String> {
    if !ss_total.is_finite() || ss_total <= 0.0 {
        return Err("eta squared is undefined because total variance is zero".into());
    }

    Ok(ss_between / ss_total)
}

fn omega_squared(
    ss_between: f64,
    ms_within: f64,
    df_between: f64,
    ss_total: f64,
) -> Result<f64, String> {
    if !ss_total.is_finite() || ss_total <= 0.0 {
        return Err("omega squared is undefined because total variance is zero".into());
    }

    let denominator = ss_total + ms_within;

    if denominator <= 0.0 || !denominator.is_finite() {
        return Err("omega squared denominator is invalid".into());
    }

    let numerator = ss_between - df_between * ms_within;

    Ok(numerator / denominator)
}

fn anova_result(summary: &AnovaSummary) -> Result<Value, String> {
    let eta2 = eta_squared(summary.ss_between, summary.ss_total)?;

    let ms_within = summary.ss_within / summary.df_within;

    let omega2 = omega_squared(
        summary.ss_between,
        ms_within,
        summary.df_between,
        summary.ss_total,
    )?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(summary.statistic)),
        ("p_value", Value::Float(summary.p_value)),
        ("df_between", Value::Float(summary.df_between)),
        ("df_within", Value::Float(summary.df_within)),
        ("effect_size", Value::Float(eta2)),
        (
            "effect_size_name",
            Value::Str(Rc::new("Eta squared".to_string())),
        ),
        ("alternative_effect_size", Value::Float(omega2)),
        (
            "alternative_effect_size_name",
            Value::Str(Rc::new("Omega squared".to_string())),
        ),
        ("confidence_interval", Value::Null),
        ("effect_size_ci", Value::Null),
        ("method", Value::Str(Rc::new("One-way ANOVA".to_string()))),
    ]))
}

pub fn anova(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("anova() expects DataFrame, response column, and factor column".into());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),

        other => {
            return Err(format!(
                "anova() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        },
    };

    let response_name = match &args[1] {
        Value::Str(value) => value.as_str(),

        other => {
            return Err(format!(
                "anova() response column must be Str, got {}",
                other.type_name()
            ));
        },
    };

    let factor_name = match &args[2] {
        Value::Str(value) => value.as_str(),

        other => {
            return Err(format!(
                "anova() factor column must be Str, got {}",
                other.type_name()
            ));
        },
    };

    let response = df
        .column(response_name)
        .ok_or_else(|| format!("anova() unknown response column '{}'", response_name))?;

    let factor = df
        .column(factor_name)
        .ok_or_else(|| format!("anova() unknown factor column '{}'", factor_name))?;

    let groups = anova_groups(&response, &factor)?;
    let summary = one_way_anova_values(&groups)?;

    anova_result(&summary)
}

// =========================
// Tukey / post-hoc
// =========================

fn tukey_alpha_from_confidence(confidence: f64) -> Result<f64, String> {
    const EPSILON: f64 = 1e-12;

    if (confidence - 0.95).abs() < EPSILON {
        return Ok(0.05);
    }

    if (confidence - 0.99).abs() < EPSILON {
        return Ok(0.01);
    }

    Err("Tukey HSD currently supports confidence levels 0.95 and 0.99".into())
}

fn string_value(value: &Value, function: &str, name: &str) -> Result<String, String> {
    match value {
        Value::Str(value) => Ok(value.as_ref().clone()),

        other => Err(format!(
            "{}() {} must be Str, got {}",
            function,
            name,
            other.type_name()
        )),
    }
}

fn tukey_values(groups: &[AnovaGroup], confidence: f64) -> Result<Value, String> {
    if groups.len() < 2 {
        return Err("tukey() requires at least 2 groups".into());
    }

    if groups.len() > 10 {
        return Err("tukey() supports at most 10 groups".into());
    }

    if groups.iter().any(|group| group.values.len() < 2) {
        return Err("tukey() requires at least 2 observations per group".into());
    }

    validate_finite(
        &groups
            .iter()
            .flat_map(|group| group.values.iter().copied())
            .collect::<Vec<_>>(),
        "tukey",
    )?;

    let alpha = tukey_alpha_from_confidence(confidence)?;

    let group_values = groups
        .iter()
        .map(|group| group.values.clone())
        .collect::<Vec<_>>();

    let result = tukey_hsd(&group_values, alpha).map_err(|error| error.to_string())?;

    let mut group1 = Vec::with_capacity(result.comparisons.len());
    let mut group2 = Vec::with_capacity(result.comparisons.len());
    let mut mean_differences = Vec::with_capacity(result.comparisons.len());
    let mut q_statistics = Vec::with_capacity(result.comparisons.len());
    let mut p_values = Vec::with_capacity(result.comparisons.len());
    let mut ci_lowers = Vec::with_capacity(result.comparisons.len());
    let mut ci_uppers = Vec::with_capacity(result.comparisons.len());
    let mut significant = Vec::with_capacity(result.comparisons.len());

    for comparison in &result.comparisons {
        let first = groups
            .get(comparison.group_i)
            .ok_or_else(|| "tukey() group index out of bounds".to_string())?;

        let second = groups
            .get(comparison.group_j)
            .ok_or_else(|| "tukey() group index out of bounds".to_string())?;

        let mean_i = first.values.iter().sum::<f64>() / first.values.len() as f64;

        let mean_j = second.values.iter().sum::<f64>() / second.values.len() as f64;

        let mean_difference = mean_i - mean_j;

        let standard_error = (result.mse / 2.0
            * (1.0 / first.values.len() as f64 + 1.0 / second.values.len() as f64))
            .sqrt();

        let margin = result.q_critical * standard_error;

        let lower = mean_difference - margin;
        let upper = mean_difference + margin;

        group1.push(first.label.clone());
        group2.push(second.label.clone());
        mean_differences.push(Value::Float(mean_difference));
        q_statistics.push(Value::Float(comparison.q_statistic));
        p_values.push(Value::Float(comparison.p_value));
        ci_lowers.push(Value::Float(lower));
        ci_uppers.push(Value::Float(upper));
        significant.push(Value::Bool(comparison.significant));
    }

    DataFrame::from_series(vec![
        Rc::new(Series::new("group1", group1)),
        Rc::new(Series::new("group2", group2)),
        Rc::new(Series::new("mean_diff", mean_differences)),
        Rc::new(Series::new("q", q_statistics)),
        Rc::new(Series::new("p_value", p_values)),
        Rc::new(Series::new("ci_lower", ci_lowers)),
        Rc::new(Series::new("ci_upper", ci_uppers)),
        Rc::new(Series::new("significant", significant)),
    ])
    .map(|df| Value::DataFrame(Rc::new(df)))
}

fn tukey_from_dataframe(
    df: &DataFrame,
    response_name: &str,
    factor_name: &str,
    confidence: f64,
) -> Result<Value, String> {
    let response = df
        .column(response_name)
        .ok_or_else(|| format!("tukey() unknown response column '{}'", response_name))?;

    let factor = df
        .column(factor_name)
        .ok_or_else(|| format!("tukey() unknown factor column '{}'", factor_name))?;

    let groups = collect_anova_groups(&response, &factor)?;

    tukey_values(&groups, confidence)
}

pub fn tukey(args: Vec<Value>) -> Result<Value, String> {
    if !(3..=4).contains(&args.len()) {
        return Err(
            "tukey() expects DataFrame, response column, factor column, and optional confidence level"
                .into(),
        );
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),

        other => {
            return Err(format!(
                "tukey() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        },
    };

    let response_name = string_value(&args[1], "tukey", "response column")?;
    let factor_name = string_value(&args[2], "tukey", "factor column")?;
    let confidence = confidence_level_from_args(&args, 3, "tukey")?;

    tukey_from_dataframe(&df, &response_name, &factor_name, confidence)
}

pub fn post_hoc(args: Vec<Value>) -> Result<Value, String> {
    if !(3..=5).contains(&args.len()) {
        return Err(
            "post_hoc() expects DataFrame, response column, factor column, optional method, and optional confidence level"
                .into(),
        );
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),

        other => {
            return Err(format!(
                "post_hoc() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        },
    };

    let response_name = string_value(&args[1], "post_hoc", "response column")?;
    let factor_name = string_value(&args[2], "post_hoc", "factor column")?;

    let method = match args.get(3) {
        None => "tukey".to_string(),

        Some(Value::Str(value)) => value.as_ref().clone(),

        Some(other) => {
            return Err(format!(
                "post_hoc() method must be Str, got {}",
                other.type_name()
            ));
        },
    };

    let confidence = match args.len() {
        3 => 0.95,
        4 => 0.95,
        _ => confidence_level_from_args(&args, 4, "post_hoc")?,
    };

    match method.as_str() {
        "tukey" | "tukey_hsd" => {
            tukey_from_dataframe(&df, &response_name, &factor_name, confidence)
        },

        other => Err(format!(
            "post_hoc() unknown method '{}'; supported methods: tukey",
            other
        )),
    }
}

// =========================
// chi-squared
// =========================

fn chi_square_independence_values(
    table: &[Vec<usize>],
) -> Result<(f64, f64, usize, usize), String> {
    let rows = table.len();

    if rows < 2 {
        return Err("chi_square() requires at least 2 rows".into());
    }

    let columns = table[0].len();

    if columns < 2 {
        return Err("chi_square() requires at least 2 columns".into());
    }

    if table.iter().any(|row| row.len() != columns) {
        return Err("chi_square() contingency table must be rectangular".into());
    }

    let mut row_totals = vec![0usize; rows];
    let mut column_totals = vec![0usize; columns];
    let mut total = 0usize;

    for row in 0..rows {
        for column in 0..columns {
            let value = table[row][column];

            row_totals[row] += value;
            column_totals[column] += value;
            total += value;
        }
    }

    if total == 0 {
        return Err("chi_square() contingency table is empty".into());
    }

    let total_f = total as f64;
    let mut statistic = 0.0;

    for row in 0..rows {
        for column in 0..columns {
            let expected = row_totals[row] as f64 * column_totals[column] as f64 / total_f;

            if expected == 0.0 {
                continue;
            }

            let observed = table[row][column] as f64;
            let difference = observed - expected;

            statistic += difference * difference / expected;
        }
    }

    let df = (rows - 1) * (columns - 1);

    let distribution = ChiSquared::new(df as f64).map_err(|error| error.to_string())?;

    let p_value = distribution.sf(statistic);

    Ok((statistic, p_value, df, total))
}

fn chi_square_table(first: &SeriesRef, second: &SeriesRef) -> Result<Vec<Vec<usize>>, String> {
    if first.len() != second.len() {
        return Err("chi_square() requires equal-length columns".into());
    }

    let mut row_keys = Vec::<CategoryKey>::new();
    let mut column_keys = Vec::<CategoryKey>::new();

    let mut row_index = HashMap::<CategoryKey, usize>::new();
    let mut column_index = HashMap::<CategoryKey, usize>::new();

    let mut observations = Vec::<(usize, usize)>::new();

    for index in 0..first.len() {
        let first_value = first
            .get(index)
            .ok_or_else(|| "first column index out of bounds".to_string())?;

        let second_value = second
            .get(index)
            .ok_or_else(|| "second column index out of bounds".to_string())?;

        let Some(first_key) = category_key(&first_value)? else {
            continue;
        };

        let Some(second_key) = category_key(&second_value)? else {
            continue;
        };

        let first_position = if let Some(position) = row_index.get(&first_key) {
            *position
        } else {
            let position = row_keys.len();

            row_index.insert(first_key.clone(), position);
            row_keys.push(first_key);

            position
        };

        let second_position = if let Some(position) = column_index.get(&second_key) {
            *position
        } else {
            let position = column_keys.len();

            column_index.insert(second_key.clone(), position);
            column_keys.push(second_key);

            position
        };

        observations.push((first_position, second_position));
    }

    if row_keys.len() < 2 || column_keys.len() < 2 {
        return Err("chi_square() requires at least 2 categories in each variable".into());
    }

    let mut table = vec![vec![0usize; column_keys.len()]; row_keys.len()];

    for (row, column) in observations {
        table[row][column] += 1;
    }

    Ok(table)
}

fn cramers_v(statistic: f64, total_n: usize, rows: usize, columns: usize) -> Result<f64, String> {
    if total_n == 0 {
        return Err("Cramer's V requires a non-empty contingency table".into());
    }

    let minimum_dimension = (rows - 1).min(columns - 1);

    if minimum_dimension == 0 {
        return Err("Cramer's V requires at least 2 categories per dimension".into());
    }

    Ok((statistic / (total_n as f64 * minimum_dimension as f64)).sqrt())
}

pub fn chi_square(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("chi_square() expects DataFrame and two column names".into());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),

        other => {
            return Err(format!(
                "chi_square() first argument must be DataFrame, got {}",
                other.type_name()
            ));
        },
    };

    let first_name = string_value(&args[1], "chi_square", "first column name")?;
    let second_name = string_value(&args[2], "chi_square", "second column name")?;

    let first = df
        .column(&first_name)
        .ok_or_else(|| format!("chi_square() unknown column '{}'", first_name))?;

    let second = df
        .column(&second_name)
        .ok_or_else(|| format!("chi_square() unknown column '{}'", second_name))?;

    let table = chi_square_table(&first, &second)?;

    let (statistic, p_value, df_value, total_n) = chi_square_independence_values(&table)?;

    let rows = table.len();
    let columns = table[0].len();

    let effect_size = cramers_v(statistic, total_n, rows, columns)?;

    Ok(result_dict(vec![
        ("statistic", Value::Float(statistic)),
        ("p_value", Value::Float(p_value)),
        ("df", Value::Int(df_value as i64)),
        ("effect_size", Value::Float(effect_size)),
        (
            "effect_size_name",
            Value::Str(Rc::new("Cramer's V".to_string())),
        ),
        ("confidence_interval", Value::Null),
        ("effect_size_ci", Value::Null),
        (
            "method",
            Value::Str(Rc::new("Chi-square test of independence".to_string())),
        ),
    ]))
}

// =========================
// regression
// =========================
struct RegressionFit {
    coefficients: Vec<f64>,
    fitted: Vec<f64>,
    residuals: Vec<f64>,
    residual_sum_of_squares: f64,
    total_sum_of_squares: f64,
    df_model: usize,
    df_residual: usize,
}

fn collect_regression_data(
    response: &SeriesRef,
    predictors: &[SeriesRef],
) -> Result<(Vec<Vec<f64>>, Vec<f64>), String> {
    if predictors.is_empty() {
        return Err("regression() requires at least one predictor".into());
    }

    let n = response.len();

    for (index, predictor) in predictors.iter().enumerate() {
        if predictor.len() != n {
            return Err(format!(
                "regression() predictor {} has length {}, expected {}",
                index,
                predictor.len(),
                n
            ));
        }
    }

    let mut x_rows = Vec::with_capacity(n);
    let mut y_values = Vec::with_capacity(n);

    for row in 0..n {
        let response_value = response
            .get(row)
            .ok_or_else(|| format!("response index out of bounds: {}", row))?;

        let response = match response_value {
            Value::Null => continue,

            Value::Int(value) => value as f64,

            Value::Float(value) => {
                if !value.is_finite() {
                    return Err(format!(
                        "regression() response contains non-finite value at row {}",
                        row
                    ));
                }

                value
            },

            other => {
                return Err(format!(
                    "regression() response must be numeric; row {} contains {}",
                    row,
                    other.type_name()
                ));
            },
        };

        let mut x_row = Vec::with_capacity(predictors.len());
        let mut complete = true;

        for predictor in predictors {
            let value = predictor
                .get(row)
                .ok_or_else(|| format!("predictor index out of bounds: {}", row))?;

            match value {
                Value::Null => {
                    complete = false;
                    break;
                },

                Value::Int(value) => x_row.push(value as f64),

                Value::Float(value) => {
                    if !value.is_finite() {
                        return Err(format!(
                            "regression() predictor '{}' contains non-finite value at row {}",
                            predictor.name(),
                            row
                        ));
                    }

                    x_row.push(value);
                },

                other => {
                    return Err(format!(
                        "regression() predictor '{}' must be numeric; row {} contains {}",
                        predictor.name(),
                        row,
                        other.type_name()
                    ));
                },
            }
        }

        if !complete {
            continue;
        }

        x_rows.push(x_row);
        y_values.push(response);
    }

    Ok((x_rows, y_values))
}

fn build_regression_design(predictor_rows: &[Vec<f64>]) -> Result<Matrix, String> {
    if predictor_rows.is_empty() {
        return Err("regression() requires at least one complete observation".into());
    }

    let predictor_count = predictor_rows[0].len();

    if predictor_count == 0 {
        return Err("regression() requires at least one predictor".into());
    }

    if predictor_rows
        .iter()
        .any(|row| row.len() != predictor_count)
    {
        return Err("regression() predictor rows have inconsistent lengths".into());
    }

    let rows = predictor_rows.len();
    let cols = predictor_count + 1;

    let mut data = Vec::with_capacity(rows * cols);

    for row in predictor_rows {
        data.push(1.0);

        data.extend(row.iter().copied());
    }

    Matrix::new(rows, cols, data)
}

fn fit_regression(x: &Matrix, y: &[f64]) -> Result<RegressionFit, String> {
    let n = x.rows();
    let parameter_count = x.cols();

    if n != y.len() {
        return Err(format!(
            "regression() X and y have incompatible row counts: {} vs {}",
            n,
            y.len()
        ));
    }

    if parameter_count < 2 {
        return Err("regression() requires at least one predictor".into());
    }

    if n == 0 {
        return Err("regression() requires at least one observation".into());
    }

    let df_model = parameter_count - 1;

    if n <= parameter_count {
        return Err(format!(
            "regression() requires more observations than parameters: {} observations, {} parameters",
            n, parameter_count
        ));
    }

    validate_regression_rank(x)?;

    let y_matrix = Mat::from_fn(n, 1, |row, _| y[row]);

    let qr = x.as_faer().qr();

    let coefficients_matrix = qr.solve_lstsq(y_matrix.as_ref());

    if coefficients_matrix.nrows() != parameter_count || coefficients_matrix.ncols() != 1 {
        return Err("regression() failed to compute coefficient vector".into());
    }

    let mut coefficients = Vec::with_capacity(parameter_count);

    for row in 0..parameter_count {
        let value = coefficients_matrix[(row, 0)];

        if !value.is_finite() {
            return Err("regression() coefficient is non-finite".into());
        }

        coefficients.push(value);
    }

    let mut fitted = Vec::with_capacity(n);
    let mut residuals = Vec::with_capacity(n);

    for row in 0..n {
        let mut fitted_value = 0.0;

        for column in 0..parameter_count {
            let x_value = x
                .get(row, column)
                .ok_or_else(|| format!("regression() failed to access X[{}, {}]", row, column))?;

            fitted_value += x_value * coefficients[column];
        }

        let residual = y[row] - fitted_value;

        if !fitted_value.is_finite() || !residual.is_finite() {
            return Err("regression() produced a non-finite fitted value".into());
        }

        fitted.push(fitted_value);
        residuals.push(residual);
    }

    let mean = y.iter().sum::<f64>() / n as f64;

    let residual_sum_of_squares = residuals.iter().map(|value| value * value).sum::<f64>();

    let total_sum_of_squares = y
        .iter()
        .map(|value| {
            let difference = *value - mean;
            difference * difference
        })
        .sum::<f64>();

    let df_residual = n - parameter_count;

    Ok(RegressionFit {
        coefficients,
        fitted,
        residuals,
        residual_sum_of_squares,
        total_sum_of_squares,
        df_model,
        df_residual,
    })
}

fn validate_regression_rank(x: &Matrix) -> Result<(), String> {
    let qr = x.as_faer().qr();
    let r = qr.thin_R();

    let dimension = x.cols();

    if r.nrows() != dimension || r.ncols() != dimension {
        return Err("regression() failed to inspect QR factor".into());
    }

    let mut max_diagonal: f64 = 0.0;

    for index in 0..dimension {
        let value = r[(index, index)].abs();

        if !value.is_finite() {
            return Err("regression() design matrix contains non-finite values".into());
        }

        max_diagonal = max_diagonal.max(value);
    }

    if max_diagonal == 0.0 {
        return Err("regression() design matrix is rank deficient".into());
    }

    let tolerance = max_diagonal * (x.rows().max(x.cols()) as f64) * f64::EPSILON * 100.0;

    for index in 0..dimension {
        if r[(index, index)].abs() <= tolerance {
            return Err("regression() design matrix is rank deficient".into());
        }
    }

    Ok(())
}

fn regression_covariance(
    x: &Matrix,
    residual_sum_of_squares: f64,
    df_residual: usize,
) -> Result<Mat<f64>, String> {
    if df_residual == 0 {
        return Err("regression() residual degrees of freedom must be positive".into());
    }

    if !residual_sum_of_squares.is_finite() || residual_sum_of_squares < 0.0 {
        return Err("regression() residual sum of squares is invalid".into());
    }

    let qr = x.as_faer().qr();
    let r = qr.thin_R().to_owned();

    let r_inverse = r.partial_piv_lu().inverse();

    let mse = residual_sum_of_squares / df_residual as f64;

    if !mse.is_finite() || mse < 0.0 {
        return Err("regression() residual mean square is invalid".into());
    }

    let covariance = (&r_inverse * r_inverse.transpose()) * mse;

    for row in 0..covariance.nrows() {
        for column in 0..covariance.ncols() {
            if !covariance[(row, column)].is_finite() {
                return Err("regression() covariance matrix is non-finite".into());
            }
        }
    }

    Ok(covariance)
}

fn regression_standard_errors(covariance: &Mat<f64>) -> Result<Vec<f64>, String> {
    if covariance.nrows() != covariance.ncols() {
        return Err("regression() covariance matrix must be square".into());
    }

    let mut standard_errors = Vec::with_capacity(covariance.nrows());

    for index in 0..covariance.nrows() {
        let variance = covariance[(index, index)];

        if !variance.is_finite() || variance < 0.0 {
            return Err(format!(
                "regression() invalid coefficient variance at index {}",
                index
            ));
        }

        standard_errors.push(variance.sqrt());
    }

    Ok(standard_errors)
}

fn regression_model_statistics(
    fit: &RegressionFit,
) -> Result<
    (
        f64, // r_squared
        f64, // adjusted_r_squared
        f64, // mse
        f64, // residual_standard_error
        f64, // f_statistic
        f64, // f_p_value
    ),
    String,
> {
    let n = fit.fitted.len();

    if n == 0 {
        return Err("regression() has no observations".into());
    }

    if fit.total_sum_of_squares == 0.0 {
        return Err("regression() response has zero total variance".into());
    }

    if fit.df_residual == 0 {
        return Err("regression() residual degrees of freedom must be positive".into());
    }

    let r_squared = 1.0 - fit.residual_sum_of_squares / fit.total_sum_of_squares;

    let adjusted_r_squared =
        1.0 - (1.0 - r_squared) * (n.saturating_sub(1) as f64) / fit.df_residual as f64;

    let mse = fit.residual_sum_of_squares / fit.df_residual as f64;

    let residual_standard_error = mse.sqrt();

    let regression_sum_of_squares = fit.total_sum_of_squares - fit.residual_sum_of_squares;

    let f_statistic = if fit.df_model == 0 {
        return Err("regression() requires at least one predictor".into());
    } else {
        let mean_regression_square = regression_sum_of_squares / fit.df_model as f64;

        if mse == 0.0 {
            if mean_regression_square == 0.0 {
                return Err("regression() has zero residual and regression variance".into());
            }

            f64::INFINITY
        } else {
            mean_regression_square / mse
        }
    };

    let f_p_value = if f_statistic.is_infinite() {
        0.0
    } else {
        let distribution = FisherSnedecor::new(fit.df_model as f64, fit.df_residual as f64)
            .map_err(|error| error.to_string())?;

        distribution.sf(f_statistic)
    };

    Ok((
        r_squared,
        adjusted_r_squared,
        mse,
        residual_standard_error,
        f_statistic,
        f_p_value,
    ))
}

fn regression_coefficient_table(
    predictor_names: &[String],
    coefficients: &[f64],
    standard_errors: &[f64],
    df_residual: usize,
    confidence: f64,
) -> Result<Value, String> {
    if coefficients.len() != standard_errors.len() {
        return Err("regression() coefficient and standard-error lengths differ".into());
    }

    if coefficients.len() != predictor_names.len() + 1 {
        return Err("regression() coefficient count does not match predictors".into());
    }

    let mut terms = Vec::with_capacity(coefficients.len());
    let mut estimates = Vec::with_capacity(coefficients.len());
    let mut errors = Vec::with_capacity(coefficients.len());
    let mut statistics = Vec::with_capacity(coefficients.len());
    let mut p_values = Vec::with_capacity(coefficients.len());
    let mut ci_lowers = Vec::with_capacity(coefficients.len());
    let mut ci_uppers = Vec::with_capacity(coefficients.len());

    for index in 0..coefficients.len() {
        let term = if index == 0 {
            "Intercept".to_string()
        } else {
            predictor_names[index - 1].clone()
        };

        let estimate = coefficients[index];
        let standard_error = standard_errors[index];

        if standard_error == 0.0 {
            if estimate == 0.0 {
                return Err(format!(
                    "regression() coefficient '{}' has undefined statistic",
                    term
                ));
            }

            let statistic = if estimate > 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };

            let p_value = 0.0;

            terms.push(Value::Str(Rc::new(term)));
            estimates.push(Value::Float(estimate));
            errors.push(Value::Float(standard_error));
            statistics.push(Value::Float(statistic));
            p_values.push(Value::Float(p_value));

            ci_lowers.push(Value::Float(estimate));
            ci_uppers.push(Value::Float(estimate));

            continue;
        }

        let statistic = estimate / standard_error;
        let p_value = t_distribution_p_value(statistic, df_residual as f64)?;

        let (ci_lower, ci_upper) =
            t_confidence_interval(estimate, standard_error, df_residual as f64, confidence)?;

        terms.push(Value::Str(Rc::new(term)));
        estimates.push(Value::Float(estimate));
        errors.push(Value::Float(standard_error));
        statistics.push(Value::Float(statistic));
        p_values.push(Value::Float(p_value));
        ci_lowers.push(Value::Float(ci_lower));
        ci_uppers.push(Value::Float(ci_upper));
    }

    let dataframe = DataFrame::from_series(vec![
        Rc::new(Series::new("term", terms)),
        Rc::new(Series::new("estimate", estimates)),
        Rc::new(Series::new("std_error", errors)),
        Rc::new(Series::new("statistic", statistics)),
        Rc::new(Series::new("p_value", p_values)),
        Rc::new(Series::new("ci_lower", ci_lowers)),
        Rc::new(Series::new("ci_upper", ci_uppers)),
    ])?;

    Ok(Value::DataFrame(Rc::new(dataframe)))
}

fn regression_numeric_series(name: &str, values: &[f64]) -> SeriesRef {
    Rc::new(Series::new(
        name.to_string(),
        values.iter().copied().map(Value::Float).collect(),
    ))
}

pub fn regression(args: Vec<Value>) -> Result<Value, String> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "regression() expects response, predictors, and optional confidence level".into(),
        );
    }

    let response = match &args[0] {
        Value::Series(series) => series.clone(),

        other => {
            return Err(format!(
                "regression() response must be Series, got {}",
                other.type_name()
            ));
        },
    };

    let predictors = expect_regression_predictors(&args[1], "regression")?;

    let confidence = confidence_level_from_args(&args, 2, "regression")?;

    let predictor_names = predictors
        .iter()
        .map(|predictor| predictor.name().to_owned())
        .collect::<Vec<_>>();

    let (predictor_rows, y) = collect_regression_data(&response, &predictors)?;

    if y.len() < 2 {
        return Err("regression() requires at least 2 complete observations".into());
    }

    let x = build_regression_design(&predictor_rows)?;

    validate_regression_rank(&x)?;

    let fit = fit_regression(&x, &y)?;

    let covariance = regression_covariance(&x, fit.residual_sum_of_squares, fit.df_residual)?;

    let standard_errors = regression_standard_errors(&covariance)?;

    let (r_squared, adjusted_r_squared, _mse, residual_standard_error, f_statistic, f_p_value) =
        regression_model_statistics(&fit)?;

    let coefficients = regression_coefficient_table(
        &predictor_names,
        &fit.coefficients,
        &standard_errors,
        fit.df_residual,
        confidence,
    )?;

    Ok(result_dict(vec![
        ("coefficients", coefficients),
        (
            "fitted",
            Value::Series(regression_numeric_series("fitted", &fit.fitted)),
        ),
        (
            "residuals",
            Value::Series(regression_numeric_series("residual", &fit.residuals)),
        ),
        ("r_squared", Value::Float(r_squared)),
        ("adjusted_r_squared", Value::Float(adjusted_r_squared)),
        ("f_statistic", Value::Float(f_statistic)),
        ("f_p_value", Value::Float(f_p_value)),
        ("df_model", Value::Int(fit.df_model as i64)),
        ("df_residual", Value::Int(fit.df_residual as i64)),
        (
            "residual_standard_error",
            Value::Float(residual_standard_error),
        ),
        (
            "residual_sum_of_squares",
            Value::Float(fit.residual_sum_of_squares),
        ),
        (
            "total_sum_of_squares",
            Value::Float(fit.total_sum_of_squares),
        ),
        ("n", Value::Int(y.len() as i64)),
        ("confidence_level", Value::Float(confidence)),
        (
            "method",
            Value::Str(Rc::new("Ordinary least squares".to_string())),
        ),
    ]))
}
