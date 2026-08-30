use crate::runtime::{
    DataFrame,
    Module,
    ModuleRef,
    Series,
    SeriesRef,
    Value,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use statrs::{
    distribution::{
        ContinuousCDF,
        FisherSnedecor,
        StudentsT,
        ChiSquared,
        Normal,
    },
    statistics::Statistics,
};

pub fn module() -> ModuleRef {
    let mut module =
        Module::new("stats");

    module.set_exported(
        "mean",
        Value::Builtin(mean),
    );

    module.set_exported(
        "variance",
        Value::Builtin(variance),
    );

    module.set_exported(
        "std",
        Value::Builtin(std),
    );

    module.set_exported(
        "median",
        Value::Builtin(median),
    );

    module.set_exported(
        "correlation",
        Value::Builtin(correlation),
    );

    module.set_exported(
        "ttest",
        Value::Builtin(ttest),
    );

    module.set_exported(
        "welch",
        Value::Builtin(welch),
    );

    // module.set_exported(
    //     "anova",
    //     Value::Builtin(anova),
    // );

    // module.set_exported(
    //     "mann_whitney",
    //     Value::Builtin(mann_whitney),
    // );

    Rc::new(
        RefCell::new(module)
    )
}


fn series_values(
    value: &Value,
) -> Result<Vec<f64>, String> {
    match value {
        Value::Series(series) =>
            series.numeric_values(),

        other => {
            Err(format!(
                "stats function expects Series, got {}",
                other.type_name()
            ))
        }
    }
}

fn dataframe_column(
    value: &Value,
    name: &str,
) -> Result<SeriesRef, String> {
    match value {
        Value::DataFrame(df) => {
            df.column(name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        name
                    )
                })
        }

        other => {
            Err(format!(
                "expected DataFrame, got {}",
                other.type_name()
            ))
        }
    }
}

fn result_dict(
    fields: Vec<(
        &str,
        Value,
    )>,
) -> Value {
    let map =
        fields
            .into_iter()
            .map(
                |(key, value)| {
                    (
                        key.to_string(),
                        value,
                    )
                }
            )
            .collect::<HashMap<_, _>>();

    Value::Dict(
        Rc::new(
            RefCell::new(map)
        )
    )
}

pub fn mean(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "mean() expects exactly 1 argument"
                .into()
        );
    }

    let values =
        series_values(
            &args[0]
        )?;

    if values.is_empty() {
        return Ok(
            Value::Null
        );
    }

    Ok(
        Value::Float(
            values.as_slice().mean()
        )
    )
}

pub fn median(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "median() expects exactly 1 argument"
                .into()
        );
    }

    let mut values =
        series_values(
            &args[0]
        )?;

    if values.is_empty() {
        return Ok(
            Value::Null
        );
    }

    values.sort_by(
        |a, b| a.total_cmp(b)
    );

    let n =
        values.len();

    let median =
        if n % 2 == 1 {
            values[n / 2]
        } else {
            (
                values[n / 2 - 1]
                    + values[n / 2]
            ) / 2.0
        };

    Ok(
        Value::Float(
            median
        )
    )
}

pub fn variance(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "variance() expects exactly 1 argument"
                .into()
        );
    }

    let values =
        series_values(
            &args[0]
        )?;

    if values.len() < 2 {
        return Ok(
            Value::Null
        );
    }

    Ok(
        Value::Float(
            values.as_slice().variance()
        )
    )
}

pub fn std(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "std() expects exactly 1 argument"
                .into()
        );
    }

    let values =
        series_values(
            &args[0]
        )?;

    if values.len() < 2 {
        return Ok(
            Value::Null
        );
    }

    Ok(
        Value::Float(
            values.as_slice().std_dev()
        )
    )
}

pub fn correlation(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "correlation() expects exactly 2 Series"
                .into()
        );
    }

    let x =
        series_values(
            &args[0]
        )?;

    let y =
        series_values(
            &args[1]
        )?;

    if x.len()
        != y.len()
    {
        return Err(
            "correlation() requires equal-length Series"
                .into()
        );
    }

    if x.len() < 2 {
        return Ok(
            Value::Null
        );
    }

    let mean_x =
        x.iter().sum::<f64>()
            / x.len() as f64;

    let mean_y =
        y.iter().sum::<f64>()
            / y.len() as f64;

    let mut numerator =
        0.0;

    let mut xx =
        0.0;

    let mut yy =
        0.0;

    for i in 0..x.len() {
        let dx =
            x[i] - mean_x;

        let dy =
            y[i] - mean_y;

        numerator +=
            dx * dy;

        xx +=
            dx * dx;

        yy +=
            dy * dy;
    }

    if xx == 0.0
        || yy == 0.0
    {
        return Ok(
            Value::Float(
                f64::NAN
            )
        );
    }

    Ok(
        Value::Float(
            numerator
                / (xx * yy).sqrt()
        )
    )
}

pub fn ttest(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "ttest() expects Series and mu0"
                .into()
        );
    }

    let values =
        series_values(
            &args[0]
        )?;

    let mu0 =
        match &args[1] {
            Value::Int(v) =>
                *v as f64,

            Value::Float(v) =>
                *v,

            other => {
                return Err(format!(
                    "ttest() mu0 must be numeric, got {}",
                    other.type_name()
                ));
            }
        };

    let n =
        values.len();

    if n < 2 {
        return Err(
            "ttest() requires at least 2 observations"
                .into()
        );
    }

    let mean =
        values
            .iter()
            .sum::<f64>()
            / n as f64;

    let variance =
        values
            .iter()
            .map(
                |x| {
                    let d =
                        *x - mean;

                    d * d
                }
            )
            .sum::<f64>()
            / (n - 1) as f64;

    let std =
        variance.sqrt();

    if std == 0.0 {
        return Err(
            "ttest() sample standard deviation is zero"
                .into()
        );
    }

    let t =
        (mean - mu0)
            / (std / (n as f64).sqrt());

    let df =
        (n - 1) as f64;

    let distribution =
        StudentsT::new(
            0.0,
            1.0,
            df,
        )
        .map_err(
            |error| {
                error.to_string()
            }
        )?;

    let p_value =
        2.0
            * distribution
                .sf(t.abs());

    Ok(
        result_dict(vec![
            (
                "statistic",
                Value::Float(t),
            ),
            (
                "p_value",
                Value::Float(
                    p_value
                ),
            ),
            (
                "df",
                Value::Float(df),
            ),
            (
                "method",
                Value::Str(
                    Rc::new(
                        "One-sample t-test"
                            .to_string()
                    )
                ),
            ),
        ])
    )
}

pub fn welch(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "welch() expects exactly 2 Series"
                .into()
        );
    }

    let x =
        series_values(
            &args[0]
        )?;

    let y =
        series_values(
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
            .map(
                |v| {
                    (*v - mean_x).powi(2)
                }
            )
            .sum::<f64>()
            / (nx - 1.0);

    let var_y =
        y.iter()
            .map(
                |v| {
                    (*v - mean_y).powi(2)
                }
            )
            .sum::<f64>()
            / (ny - 1.0);

    let se2 =
        var_x / nx
        + var_y / ny;

    if se2 == 0.0 {
        return Err(
            "welch() standard error is zero"
                .into()
        );
    }

    let t =
        (mean_x - mean_y)
            / se2.sqrt();

    let numerator =
        se2.powi(2);

    let denominator =
        (var_x.powi(2)
            / (nx.powi(2) * (nx - 1.0)))
        +
        (var_y.powi(2)
            / (ny.powi(2) * (ny - 1.0)));

    let df =
        numerator / denominator;

    let distribution =
        StudentsT::new(
            0.0,
            1.0,
            df,
        )
        .map_err(
            |error| {
                error.to_string()
            }
        )?;

    let p_value =
        2.0
            * distribution
                .sf(t.abs());

    Ok(
        result_dict(vec![
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
                "method",
                Value::Str(
                    Rc::new(
                        "Welch's t-test"
                            .to_string()
                    )
                ),
            ),
        ])
    )
}



