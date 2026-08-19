use crate::runtime::{
    Object,
    Value,
};

use std::{
    cell::RefCell,
    rc::Rc,
};

use super::{
    descriptive::collect_numbers,
    distribution::{
        normal_cdf,
        student_t_cdf,
    },
};

fn result_object(
    test: &str,
    statistic: f64,
    p_value: f64,
) -> Value {
    let mut object = Object::new();

    object.set_type_name(
        "TestResult"
    );

    object.set_field(
        "test",
        Value::Str(
            Rc::new(test.to_string())
        ),
    );

    object.set_field(
        "statistic",
        Value::Float(statistic),
    );

    object.set_field(
        "p_value",
        Value::Float(p_value),
    );

    Value::Object(
        Rc::new(
            RefCell::new(object)
        )
    )
}

pub fn one_sample_t(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "one_sample_t() expects 2 arguments".into()
        );
    }

    let x =
        collect_numbers(&args[0])?;

    let mu =
        match args[1] {
            Value::Int(v) => v as f64,
            Value::Float(v) => v,

            _ => {
                return Err(
                    "one_sample_t() population mean must be numeric"
                        .into()
                );
            }
        };

    if x.len() < 2 {
        return Err(
            "one_sample_t() requires at least 2 observations"
                .into()
        );
    }

    let n = x.len() as f64;

    let mean =
        x.iter().sum::<f64>() / n;

    let variance =
        x.iter()
            .map(|v| {
                let d = *v - mean;
                d * d
            })
            .sum::<f64>()
            / (n - 1.0);

    let se =
        (variance / n).sqrt();

    let df = n - 1.0;

    let (t, p) = if se == 0.0 {
        if (mean - mu).abs() <= 1e-15 {
            // The sample is constant and exactly equal
            // to the null-hypothesized mean.
            //
            // This is a degenerate but non-contradictory
            // case: t = 0 and p = 1.
            (0.0, 1.0)
        } else {
            // The sample is constant but differs from the
            // null-hypothesized mean.
            //
            // The standard error tends to zero, so the
            // test statistic tends to +/- infinity and
            // the two-sided p-value tends to 0.
            let t = if mean > mu {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };

            (t, 0.0)
        }
    } else {
        let t =
            (mean - mu) / se;

        let p =
            2.0 * (
                1.0
                    - student_t_cdf(
                        t.abs(),
                        df,
                    )
            );

        (t, p)
    };

    let result =
        result_object(
            "one-sample t-test",
            t,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "df",
            Value::Float(df),
        );

        object.borrow_mut().set_field(
            "mean",
            Value::Float(mean),
        );

        object.borrow_mut().set_field(
            "mu",
            Value::Float(mu),
        );
    }

    Ok(result)
}

pub fn paired_t(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "paired_t() expects 2 arguments".into()
        );
    }

    let x =
        collect_numbers(&args[0])?;

    let y =
        collect_numbers(&args[1])?;

    if x.len() != y.len() {
        return Err(
            "paired_t() requires equal sample sizes"
                .into()
        );
    }

    let differences =
        x.iter()
            .zip(y.iter())
            .map(|(x, y)| Value::Float(*x - *y))
            .collect::<Vec<_>>();

    let diff_list =
        Value::List(
            Rc::new(
                RefCell::new(
                    differences
                )
            )
        );

    let result =
        one_sample_t(vec![
            diff_list,
            Value::Float(0.0),
        ])?;

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "test",
            Value::Str(
                Rc::new(
                    "paired t-test".to_string()
                )
            ),
        );
    }

    Ok(result)
}

pub fn welch_t(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "welch_t() expects 2 arguments".into()
        );
    }

    let x =
        collect_numbers(&args[0])?;

    let y =
        collect_numbers(&args[1])?;

    if x.len() < 2 || y.len() < 2 {
        return Err(
            "welch_t() requires at least 2 observations in each sample"
                .into()
        );
    }

    let nx = x.len() as f64;
    let ny = y.len() as f64;

    let mean_x =
        x.iter().sum::<f64>() / nx;

    let mean_y =
        y.iter().sum::<f64>() / ny;

    let var_x =
        x.iter()
            .map(|v| {
                let d = *v - mean_x;
                d * d
            })
            .sum::<f64>()
            / (nx - 1.0);

    let var_y =
        y.iter()
            .map(|v| {
                let d = *v - mean_y;
                d * d
            })
            .sum::<f64>()
            / (ny - 1.0);

    let se =
        (
            var_x / nx
                + var_y / ny
        ).sqrt();

    if se == 0.0 {
        let result =
            if (mean_x - mean_y).abs() <= 1e-15 {
                result_object(
                    "Welch's t-test",
                    0.0,
                    1.0,
                )
            } else {
                let statistic =
                    if mean_x > mean_y {
                        f64::INFINITY
                    } else {
                        f64::NEG_INFINITY
                    };

                result_object(
                    "Welch's t-test",
                    statistic,
                    0.0,
                )
            };

        return Ok(result);
    }

    let statistic =
        (mean_x - mean_y) / se;

    let a = var_x / nx;
    let b = var_y / ny;

    let df =
        (a + b).powi(2)
            / (
                a.powi(2) / (nx - 1.0)
                    + b.powi(2) / (ny - 1.0)
            );

    let p =
        2.0 * (
            1.0
                - student_t_cdf(
                    statistic.abs(),
                    df,
                )
        );

    let result =
        result_object(
            "Welch's t-test",
            statistic,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "df",
            Value::Float(df),
        );

        object.borrow_mut().set_field(
            "mean_x",
            Value::Float(mean_x),
        );

        object.borrow_mut().set_field(
            "mean_y",
            Value::Float(mean_y),
        );
    }

    Ok(result)
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

pub fn mann_whitney(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "mann_whitney() expects 2 arguments".into()
        );
    }

    let x =
        collect_numbers(&args[0])?;

    let y =
        collect_numbers(&args[1])?;

    let nx = x.len();
    let ny = y.len();

    if nx == 0 || ny == 0 {
        return Err(
            "mann_whitney() requires non-empty samples"
                .into()
        );
    }

    let mut combined =
        Vec::with_capacity(nx + ny);

    combined.extend(
        x.iter().copied()
    );

    combined.extend(
        y.iter().copied()
    );

    let ranks =
        rank_values(&combined);

    let rank_sum_x =
        ranks[..nx]
            .iter()
            .sum::<f64>();

    let u1 =
        rank_sum_x
            - (nx * (nx + 1) / 2) as f64;

    let u2 =
        (nx * ny) as f64 - u1;

    let u =
        u1.min(u2);

    let n =
        (nx + ny) as f64;

    let mean_u =
        (nx * ny) as f64 / 2.0;

    // Tie correction
    let mut sorted =
        combined.clone();

    sorted.sort_by(
        |a, b| a.total_cmp(b)
    );

    let mut tie_sum = 0.0;
    let mut i = 0;

    while i < sorted.len() {
        let mut j = i + 1;

        while j < sorted.len()
            && sorted[j] == sorted[i]
        {
            j += 1;
        }

        let t =
            (j - i) as f64;

        if t > 1.0 {
            tie_sum +=
                t.powi(3) - t;
        }

        i = j;
    }

    let variance_u =
        (
            (nx * ny) as f64
            / 12.0
        ) * (
            n + 1.0
            - tie_sum
                / (n * (n - 1.0))
        );

    if variance_u <= 0.0 {
        return Err(
            "Mann-Whitney variance is not positive"
                .into()
        );
    }

    let correction =
        if u > mean_u {
            0.5
        } else if u < mean_u {
            -0.5
        } else {
            0.0
        };

    let z =
        (u - mean_u - correction)
            / variance_u.sqrt();

    let p =
        2.0 * (
            1.0
                - normal_cdf(z.abs())
        );

    let result =
        result_object(
            "Mann-Whitney U test (asymptotic)",
            u,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "u",
            Value::Float(u),
        );

        object.borrow_mut().set_field(
            "z",
            Value::Float(z),
        );

        object.borrow_mut().set_field(
            "n1",
            Value::Int(nx as i64),
        );

        object.borrow_mut().set_field(
            "n2",
            Value::Int(ny as i64),
        );
    }

    Ok(result)
}