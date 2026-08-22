pub fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let t = 1.0 / (1.0 + p * x);

    let y = 1.0
        - (((((a5 * t + a4) * t + a3) * t + a2) * t + a1)
            * t
            * (-x * x).exp());

    sign * y
}

pub fn ln_gamma(x: f64) -> f64 {
    const COEFF: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];

    if x < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * x).sin().ln()
            - ln_gamma(1.0 - x);
    }

    let z = x - 1.0;

    let mut a = COEFF[0];

    for (i, c) in COEFF.iter().enumerate().skip(1) {
        a += c / (z + i as f64);
    }

    let t = z + 7.5;

    0.5 * (2.0 * std::f64::consts::PI).ln()
        + (z + 0.5) * t.ln()
        - t
        + a.ln()
}

pub fn regularized_gamma_p(
    a: f64,
    x: f64,
) -> f64 {
    if a <= 0.0 || x < 0.0 {
        return f64::NAN;
    }

    if x == 0.0 {
        return 0.0;
    }

    // Series representation for x < a + 1
    if x < a + 1.0 {
        let mut sum = 1.0 / a;
        let mut term = sum;
        let mut n = 1.0;

        while n <= 1000.0 {
            term *= x / (a + n);
            sum += term;

            if term.abs() < sum.abs() * 1e-14 {
                break;
            }

            n += 1.0;
        }

        let log_prefactor =
            -x
            + a * x.ln()
            - ln_gamma(a);

        return sum * log_prefactor.exp();
    }

    // Continued fraction for x >= a + 1
    const MAX_ITER: usize = 1000;
    const EPS: f64 = 1e-14;
    const FPMIN: f64 = 1e-300;

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;

    for i in 1..=MAX_ITER {
        let i = i as f64;

        let an = -i * (i - a);

        b += 2.0;

        d = an * d + b;

        if d.abs() < FPMIN {
            d = FPMIN;
        }

        c = b + an / c;

        if c.abs() < FPMIN {
            c = FPMIN;
        }

        d = 1.0 / d;

        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < EPS {
            break;
        }
    }

    let q =
        (-x + a * x.ln() - ln_gamma(a)).exp()
        * h;

    1.0 - q
}

fn beta_continued_fraction(
    a: f64,
    b: f64,
    x: f64,
) -> f64 {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 3.0e-14;
    const FPMIN: f64 = 1.0e-300;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;

    let mut c = 1.0;

    let mut d =
        1.0 - qab * x / qap;

    if d.abs() < FPMIN {
        d = FPMIN;
    }

    d = 1.0 / d;

    let mut h = d;

    for m in 1..=MAX_ITER {
        let m = m as f64;
        let m2 = 2.0 * m;

        let mut aa =
            m * (b - m) * x
                / ((qam + m2) * (a + m2));

        d = 1.0 + aa * d;

        if d.abs() < FPMIN {
            d = FPMIN;
        }

        c = 1.0 + aa / c;

        if c.abs() < FPMIN {
            c = FPMIN;
        }

        d = 1.0 / d;
        h *= d * c;

        aa =
            -(a + m)
                * (qab + m)
                * x
                / ((a + m2) * (qap + m2));

        d = 1.0 + aa * d;

        if d.abs() < FPMIN {
            d = FPMIN;
        }

        c = 1.0 + aa / c;

        if c.abs() < FPMIN {
            c = FPMIN;
        }

        d = 1.0 / d;

        let delta = d * c;

        h *= delta;

        if (delta - 1.0).abs() < EPS {
            break;
        }
    }

    h
}

pub fn regularized_beta(
    x: f64,
    a: f64,
    b: f64,
) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }

    if x >= 1.0 {
        return 1.0;
    }

    let ln_beta =
        ln_gamma(a)
            + ln_gamma(b)
            - ln_gamma(a + b);

    let front =
        (a * x.ln()
            + b * (1.0 - x).ln()
            - ln_beta)
            .exp();

    if x < (a + 1.0) / (a + b + 2.0) {
        front
            * beta_continued_fraction(a, b, x)
            / a
    } else {
        1.0
            - front
                * beta_continued_fraction(
                    b,
                    a,
                    1.0 - x,
                )
                / b
    }
}