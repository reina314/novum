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
        student_t_quantile,
        chi_square_cdf,
        f_cdf,
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

/// ## Sample
/// ```py
/// mean_ci([1,2,3,4,5], 0.95)
/// ```
pub fn mean_ci(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "mean_ci() expects data and confidence"
                .into()
        );
    }

    let x =
        collect_numbers(&args[0])?;

    let confidence =
        match args[1] {
            Value::Int(v) => v as f64,
            Value::Float(v) => v,

            _ => {
                return Err(
                    "confidence must be numeric"
                        .into()
                );
            }
        };

    if !(0.0 < confidence && confidence < 1.0) {
        return Err(
            "confidence must be between 0 and 1"
                .into()
        );
    }

    if x.len() < 2 {
        return Err(
            "mean_ci() requires at least 2 observations"
                .into()
        );
    }

    let n =
        x.len() as f64;

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

    let alpha =
    1.0 - confidence;

    let critical =
        student_t_quantile(
            1.0 - alpha / 2.0,
            n - 1.0,
        );

    let margin =
        critical * se;

    let result =
        result_object(
            "Mean confidence interval",
            mean,
            0.0,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "mean",
            Value::Float(mean),
        );

        object.borrow_mut().set_field(
            "lower",
            Value::Float(mean - margin),
        );

        object.borrow_mut().set_field(
            "upper",
            Value::Float(mean + margin),
        );

        object.borrow_mut().set_field(
            "confidence",
            Value::Float(confidence),
        );

        object.borrow_mut().set_field(
            "df",
            Value::Float(n - 1.0),
        );
    }

    Ok(result)
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

/// Internally executes `one_sample_t()` of the difference between pairs.
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

/// Chi Squared test for Goodness of Fit
/// 
/// ## Usage
/// ```py
/// chi_square_gof(observed, expected)
/// ```
/// 
/// ## Sample
/// ```py
/// chi_square_gof(
///     [20, 30, 50],
///     [25, 25, 50]
/// )
/// ```
pub fn chi_square_gof(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "chi_square_gof() expects observed and expected"
                .into()
        );
    }

    let observed =
        collect_numbers(&args[0])?;

    let expected =
        collect_numbers(&args[1])?;

    if observed.len() != expected.len() {
        return Err(
            "observed and expected must have equal lengths"
                .into()
        );
    }

    if observed.len() < 2 {
        return Err(
            "chi-square goodness-of-fit requires at least 2 categories"
                .into()
        );
    }

    let mut statistic = 0.0;

    for (o, e) in observed
        .iter()
        .zip(expected.iter())
    {
        if *e <= 0.0 {
            return Err(
                "expected frequencies must be positive"
                    .into()
            );
        }

        statistic +=
            (o - e).powi(2) / e;
    }

    let df =
        (observed.len() - 1) as f64;

    let p =
        1.0
        - chi_square_cdf(
            statistic,
            df,
        );

    let result =
        result_object(
            "Chi-square goodness-of-fit test",
            statistic,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "df",
            Value::Float(df),
        );
    }

    Ok(result)
}

/// Chi Squared test for Independence
///  
/// ### Sample
/// ```py
/// chi_square_independence(
///     matrix([
///         [10, 20, 30],
///         [20, 30, 10]
///     ])
/// )
/// ```
pub fn chi_square_independence(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "chi_square_independence() expects a Matrix"
                .into()
        );
    }

    let matrix =
        match &args[0] {
            Value::Matrix(matrix) =>
                matrix.borrow(),

            other => {
                return Err(format!(
                    "chi_square_independence() expects Matrix, got {}",
                    other.type_name()
                ));
            }
        };

    let rows = matrix.rows();
    let cols = matrix.cols();

    if rows < 2 || cols < 2 {
        return Err(
            "contingency table must have at least 2 rows and 2 columns"
                .into()
        );
    }

    let mut row_totals =
        vec![0.0; rows];

    let mut col_totals =
        vec![0.0; cols];

    let mut total = 0.0;

    for r in 0..rows {
        for c in 0..cols {
            let value =
                matrix.get(r, c).unwrap();

            if value < 0.0 {
                return Err(
                    "observed frequencies must be non-negative"
                        .into()
                );
            }

            row_totals[r] += value;
            col_totals[c] += value;
            total += value;
        }
    }

    if total <= 0.0 {
        return Err(
            "contingency table total must be positive"
                .into()
        );
    }

    let mut statistic = 0.0;

    for r in 0..rows {
        for c in 0..cols {
            let expected =
                row_totals[r]
                    * col_totals[c]
                    / total;

            if expected > 0.0 {
                let observed =
                    matrix.get(r, c).unwrap();

                statistic +=
                    (observed - expected).powi(2)
                    / expected;
            }
        }
    }

    let df =
        ((rows - 1) * (cols - 1))
            as f64;

    let p =
        1.0
        - chi_square_cdf(
            statistic,
            df,
        );

    let result =
        result_object(
            "Chi-square test of independence",
            statistic,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "df",
            Value::Float(df),
        );

        object.borrow_mut().set_field(
            "rows",
            Value::Int(rows as i64),
        );

        object.borrow_mut().set_field(
            "cols",
            Value::Int(cols as i64),
        );
    }

    Ok(result)
}

pub fn anova(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "anova() expects a List of groups"
                .into()
        );
    }

    let groups =
        match &args[0] {
            Value::List(groups) =>
                groups.borrow(),

            other => {
                return Err(format!(
                    "anova() expects List, got {}",
                    other.type_name()
                ));
            }
        };

    if groups.len() < 2 {
        return Err(
            "ANOVA requires at least 2 groups"
                .into()
        );
    }

    let mut data =
        Vec::<Vec<f64>>::new();

    for group in groups.iter() {
        data.push(
            collect_numbers(group)?
        );
    }

    let total_n: usize =
        data.iter()
            .map(Vec::len)
            .sum();

    if total_n <= data.len() {
        return Err(
            "ANOVA requires residual degrees of freedom"
                .into()
        );
    }

    let grand_mean =
        data.iter()
            .flatten()
            .sum::<f64>()
            / total_n as f64;

    let mut ss_between = 0.0;
    let mut ss_within = 0.0;

    for group in &data {
        let n =
            group.len() as f64;

        let mean =
            group.iter().sum::<f64>()
            / n;

        ss_between +=
            n * (mean - grand_mean).powi(2);

        ss_within +=
            group
                .iter()
                .map(|x| {
                    (x - mean).powi(2)
                })
                .sum::<f64>();
    }

    let k =
        data.len() as f64;

    let df_between =
        k - 1.0;

    let df_within =
        total_n as f64 - k;

    let ms_between =
        ss_between / df_between;

    let ms_within =
        ss_within / df_within;

    if ms_within == 0.0 {
        let result =
            if ss_between == 0.0 {
                result_object(
                    "One-way ANOVA",
                    0.0,
                    1.0,
                )
            } else {
                result_object(
                    "One-way ANOVA",
                    f64::INFINITY,
                    0.0,
                )
            };

        if let Value::Object(object) = &result {
            object.borrow_mut().set_field(
                "df_between",
                Value::Float(df_between),
            );

            object.borrow_mut().set_field(
                "df_within",
                Value::Float(df_within),
            );
        }

        return Ok(result);
    }

    let f =
        ms_between / ms_within;

    let p =
        1.0
        - f_cdf(
            f,
            df_between,
            df_within,
        );

    let result =
        result_object(
            "One-way ANOVA",
            f,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "f",
            Value::Float(f),
        );

        object.borrow_mut().set_field(
            "df_between",
            Value::Float(df_between),
        );

        object.borrow_mut().set_field(
            "df_within",
            Value::Float(df_within),
        );

        object.borrow_mut().set_field(
            "ss_between",
            Value::Float(ss_between),
        );

        object.borrow_mut().set_field(
            "ss_within",
            Value::Float(ss_within),
        );

        object.borrow_mut().set_field(
            "ms_between",
            Value::Float(ms_between),
        );

        object.borrow_mut().set_field(
            "ms_within",
            Value::Float(ms_within),
        );
    }

    Ok(result)
}

pub fn kruskal_wallis(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "kruskal_wallis() expects a List of groups"
                .into()
        );
    }

    let groups =
        match &args[0] {
            Value::List(groups) =>
                groups.borrow(),

            other => {
                return Err(format!(
                    "kruskal_wallis() expects List, got {}",
                    other.type_name()
                ));
            }
        };

    if groups.len() < 2 {
        return Err(
            "Kruskal-Wallis requires at least 2 groups"
                .into()
        );
    }

    let mut samples =
        Vec::<Vec<f64>>::new();

    for group in groups.iter() {
        samples.push(
            collect_numbers(group)?
        );
    }

    let mut combined =
        Vec::<(f64, usize)>::new();

    for (group_index, group) in
        samples.iter().enumerate()
    {
        for value in group {
            combined.push(
                (*value, group_index)
            );
        }
    }

    combined.sort_by(
        |a, b| a.0.total_cmp(&b.0)
    );

    let mut ranks =
        vec![0.0; combined.len()];

    let mut i = 0;

    while i < combined.len() {
        let mut j = i + 1;

        while j < combined.len()
            && combined[j].0
                == combined[i].0
        {
            j += 1;
        }

        let rank =
            (i + 1 + j) as f64 / 2.0;

        for k in i..j {
            ranks[k] = rank;
        }

        i = j;
    }

    let mut rank_sums =
        vec![0.0; samples.len()];

    let mut counts =
        vec![0usize; samples.len()];

    for (i, (_, group)) in
        combined.iter().enumerate()
    {
        rank_sums[*group] += ranks[i];
        counts[*group] += 1;
    }

    let n =
        combined.len() as f64;

    let mut h =
        0.0;

    for i in 0..samples.len() {
        let ni =
            counts[i] as f64;

        h +=
            rank_sums[i]
                * rank_sums[i]
                / ni;
    }

    h =
        12.0 * h
        / (n * (n + 1.0))
        - 3.0 * (n + 1.0);

    let df =
        (samples.len() - 1) as f64;

    let p =
        1.0
        - chi_square_cdf(
            h.max(0.0),
            df,
        );

    let result =
        result_object(
            "Kruskal-Wallis test",
            h,
            p,
        );

    if let Value::Object(object) = &result {
        object.borrow_mut().set_field(
            "h",
            Value::Float(h),
        );

        object.borrow_mut().set_field(
            "df",
            Value::Float(df),
        );

        object.borrow_mut().set_field(
            "n",
            Value::Int(n as i64),
        );
    }

    Ok(result)
}




