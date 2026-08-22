use super::util::{
    erf,
    regularized_beta,
    regularized_gamma_p,
};

pub fn normal_cdf(
    x: f64,
) -> f64 {
    0.5 * (
        1.0
            + erf(
                x / 2.0_f64.sqrt()
            )
    )
}

pub fn student_t_cdf(
    t: f64,
    df: f64,
) -> f64 {
    if df <= 0.0 {
        return f64::NAN;
    }

    if t == 0.0 {
        return 0.5;
    }

    let x =
        df / (df + t * t);

    let ib =
        regularized_beta(
            x,
            df / 2.0,
            0.5,
        );

    if t > 0.0 {
        1.0 - 0.5 * ib
    } else {
        0.5 * ib
    }
}

/// Inverse of `student_t_cdf`
pub fn student_t_quantile(
    p: f64,
    df: f64,
) -> f64 {
    if !(0.0 < p && p < 1.0)
        || df <= 0.0
    {
        return f64::NAN;
    }

    if (p - 0.5).abs() < 1e-15 {
        return 0.0;
    }

    let sign =
        if p < 0.5 {
            -1.0
        } else {
            1.0
        };

    let target =
        if sign > 0.0 {
            p
        } else {
            1.0 - p
        };

    let mut low = 0.0;
    let mut high = 1.0;

    while student_t_cdf(high, df)
        < target
    {
        high *= 2.0;

        if high > 1.0e10 {
            return f64::INFINITY * sign;
        }
    }

    for _ in 0..100 {
        let mid =
            (low + high) / 2.0;

        if student_t_cdf(mid, df)
            < target
        {
            low = mid;
        } else {
            high = mid;
        }
    }

    sign * (low + high) / 2.0
}

pub fn chi_square_cdf(
    x: f64,
    df: f64,
) -> f64 {
    if x < 0.0 || df <= 0.0 {
        return f64::NAN;
    }

    regularized_gamma_p(
        df / 2.0,
        x / 2.0,
    )
}

pub fn f_cdf(
    x: f64,
    df1: f64,
    df2: f64,
) -> f64 {
    if x < 0.0
        || df1 <= 0.0
        || df2 <= 0.0
    {
        return f64::NAN;
    }

    let z =
        (df1 * x)
        / (df1 * x + df2);

    regularized_beta(
        z,
        df1 / 2.0,
        df2 / 2.0,
    )
}











