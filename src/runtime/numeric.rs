use faer::{
    Accum,
    Mat,
    Par,
    Scale,
};

pub const PARALLEL_THRESHOLD: usize =
    1_000_000;

#[inline]
pub fn should_parallelize(
    work: usize,
) -> bool {
    work >= PARALLEL_THRESHOLD
}

#[inline]
pub fn vector_add(
    lhs: &faer::Col<f64>,
    rhs: &faer::Col<f64>,
) -> faer::Col<f64> {
    lhs + rhs
}

#[inline]
pub fn vector_sub(
    lhs: &faer::Col<f64>,
    rhs: &faer::Col<f64>,
) -> faer::Col<f64> {
    lhs - rhs
}

#[inline]
pub fn vector_scale(
    vector: &faer::Col<f64>,
    scalar: f64,
) -> faer::Col<f64> {
    Scale(scalar) * vector
}

#[inline]
pub fn vector_dot(
    lhs: &faer::Col<f64>,
    rhs: &faer::Col<f64>,
) -> f64 {
    let mut result =
        0.0;

    for index in 0..lhs.nrows() {
        result +=
            lhs[index]
            * rhs[index];
    }

    result
}

#[inline]
pub fn vector_norm(
    vector: &faer::Col<f64>,
) -> f64 {
    vector_dot(
        vector,
        vector,
    )
    .sqrt()
}

#[inline]
pub fn matrix_add(
    lhs: &Mat<f64>,
    rhs: &Mat<f64>,
) -> Result<Mat<f64>, String> {
    if lhs.nrows()
        != rhs.nrows()
        || lhs.ncols()
            != rhs.ncols()
    {
        return Err(format!(
            "matrix shape mismatch: ({}, {}) vs ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    Ok(
        lhs + rhs
    )
}

#[inline]
pub fn matrix_sub(
    lhs: &Mat<f64>,
    rhs: &Mat<f64>,
) -> Result<Mat<f64>, String> {
    if lhs.nrows()
        != rhs.nrows()
        || lhs.ncols()
            != rhs.ncols()
    {
        return Err(format!(
            "matrix shape mismatch: ({}, {}) vs ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    Ok(
        lhs - rhs
    )
}

pub fn matrix_scale(
    matrix: &Mat<f64>,
    scalar: f64,
) -> Mat<f64> {
    Scale(scalar) * matrix
}

pub fn matrix_matmul(
    lhs: &faer::Mat<f64>,
    rhs: &faer::Mat<f64>,
) -> Result<
    faer::Mat<f64>,
    String,
> {
    if lhs.ncols()
        != rhs.nrows()
    {
        return Err(format!(
            "cannot matrix-multiply shapes ({}, {}) and ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    let rows =
        lhs.nrows();

    let cols =
        rhs.ncols();

    let inner =
        lhs.ncols();

    let mut result =
        faer::Mat::<f64>::zeros(
            rows,
            cols,
        );

    let work =
        rows
            .saturating_mul(cols)
            .saturating_mul(inner);

    let par =
        if should_parallelize(work) {
            faer::Par::rayon(0)
        } else {
            faer::Par::Seq
        };

    faer::linalg::matmul::matmul(
        &mut result,
        faer::Accum::Replace,
        lhs,
        rhs,
        1.0,
        par,
    );

    Ok(result)
}

pub fn matrix_elementwise_mul(
    lhs: &Mat<f64>,
    rhs: &Mat<f64>,
) -> Mat<f64> {
    let rows =
        lhs.nrows();

    let cols =
        lhs.ncols();

    let mut result =
        Mat::<f64>::zeros(
            rows,
            cols,
        );

    faer::zip!(
        &mut result,
        lhs,
        rhs,
    )
    .for_each(
        |faer::unzip!(
            result,
            lhs,
            rhs
        )| {
            *result =
                lhs * rhs;
        }
    );

    result
}

pub fn vector_matrix_mul(
    vector: &faer::Col<f64>,
    matrix: &faer::Mat<f64>,
) -> Result<
    faer::Col<f64>,
    String,
> {
    if vector.nrows()
        != matrix.nrows()
    {
        return Err(format!(
            "vector-matrix multiplication dimension mismatch: vector length {}, matrix shape ({}, {})",
            vector.nrows(),
            matrix.nrows(),
            matrix.ncols(),
        ));
    }

    let rows =
        matrix.nrows();

    let cols =
        matrix.ncols();

    let mut result =
        faer::Col::<f64>::zeros(
            cols
        );

    let work =
        rows.saturating_mul(cols);

    let par =
        if should_parallelize(work) {
            faer::Par::rayon(0)
        } else {
            faer::Par::Seq
        };

    for col in 0..cols {
        let mut sum =
            0.0;

        for row in 0..rows {
            sum +=
                vector[row]
                * matrix[(row, col)];
        }

        result[col] =
            sum;
    }

    let _ =
        par;

    Ok(result)
}

pub fn matrix_vector_mul(
    matrix: &Mat<f64>,
    vector: &faer::Col<f64>,
) -> Result<
    faer::Col<f64>,
    String,
> {
    if matrix.ncols()
        != vector.nrows()
    {
        return Err(format!(
            "matrix-vector multiplication dimension mismatch: matrix shape ({}, {}), vector length {}",
            matrix.nrows(),
            matrix.ncols(),
            vector.nrows(),
        ));
    }

    let mut result =
        faer::Col::<f64>::zeros(
            matrix.nrows()
        );

    let work =
        matrix
            .nrows()
            .saturating_mul(
                matrix.ncols()
            );

    let par =
        if should_parallelize(work) {
            Par::rayon(0)
        } else {
            Par::Seq
        };

    faer::linalg::matmul::matmul(
        result.as_mat_mut(),
        Accum::Replace,
        matrix,
        vector.as_mat(),
        1.0,
        par,
    );

    Ok(result)
}

pub fn matrix_determinant(
    matrix: &Mat<f64>,
) -> Result<f64, String> {
    let n =
        matrix.nrows();

    if n != matrix.ncols() {
        return Err(
            "determinant requires a square matrix"
                .into()
        );
    }

    if n == 0 {
        return Err(
            "determinant requires a non-empty matrix"
                .into()
        );
    }

    let mut a =
        matrix.clone();

    let mut determinant =
        1.0;

    let mut sign =
        1.0;

    for i in 0..n {
        let mut pivot =
            i;

        let mut pivot_abs =
            a[(i, i)].abs();

        for row in (i + 1)..n {
            let candidate =
                a[(row, i)].abs();

            if candidate > pivot_abs {
                pivot =
                    row;

                pivot_abs =
                    candidate;
            }
        }

        if pivot_abs < 1e-12 {
            return Ok(0.0);
        }

        if pivot != i {
            for col in 0..n {
                let tmp =
                    a[(i, col)];

                a[(i, col)] =
                    a[(pivot, col)];

                a[(pivot, col)] =
                    tmp;
            }

            sign =
                -sign;
        }

        let pivot_value =
            a[(i, i)];

        determinant *=
            pivot_value;

        for row in (i + 1)..n {
            let factor =
                a[(row, i)]
                    / pivot_value;

            a[(row, i)] =
                0.0;

            for col in (i + 1)..n {
                a[(row, col)] -=
                    factor
                    * a[(i, col)];
            }
        }
    }

    Ok(
        determinant * sign
    )
}