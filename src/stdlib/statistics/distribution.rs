use super::util::{
    erf,
    regularized_beta,
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