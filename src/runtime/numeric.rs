use faer::{
    linalg::solvers::{Solve, SolveLstsq},
    Accum, Mat, Par, Scale,
};

pub const PARALLEL_THRESHOLD: usize = 1_000_000;

#[inline]
pub fn should_parallelize(work: usize) -> bool {
    work >= PARALLEL_THRESHOLD
}

#[inline]
pub fn vector_add(lhs: &faer::Col<f64>, rhs: &faer::Col<f64>) -> faer::Col<f64> {
    lhs + rhs
}

#[inline]
pub fn vector_sub(lhs: &faer::Col<f64>, rhs: &faer::Col<f64>) -> faer::Col<f64> {
    lhs - rhs
}

#[inline]
pub fn vector_scale(vector: &faer::Col<f64>, scalar: f64) -> faer::Col<f64> {
    Scale(scalar) * vector
}

#[inline]
pub fn vector_dot(lhs: &faer::Col<f64>, rhs: &faer::Col<f64>) -> f64 {
    faer::linalg::matmul::dot::inner_prod(
        lhs.transpose(),
        faer::Conj::No,
        rhs.as_dyn_stride(),
        faer::Conj::No,
    )
}

#[inline]
pub fn vector_norm(vector: &faer::Col<f64>) -> f64 {
    vector_dot(vector, vector).sqrt()
}

#[inline]
pub fn matrix_add(lhs: &Mat<f64>, rhs: &Mat<f64>) -> Result<Mat<f64>, String> {
    if lhs.nrows() != rhs.nrows() || lhs.ncols() != rhs.ncols() {
        return Err(format!(
            "matrix shape mismatch: ({}, {}) vs ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    Ok(lhs + rhs)
}

#[inline]
pub fn matrix_sub(lhs: &Mat<f64>, rhs: &Mat<f64>) -> Result<Mat<f64>, String> {
    if lhs.nrows() != rhs.nrows() || lhs.ncols() != rhs.ncols() {
        return Err(format!(
            "matrix shape mismatch: ({}, {}) vs ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    Ok(lhs - rhs)
}

pub fn matrix_scale(matrix: &Mat<f64>, scalar: f64) -> Mat<f64> {
    Scale(scalar) * matrix
}

pub fn matrix_matmul(lhs: &faer::Mat<f64>, rhs: &faer::Mat<f64>) -> Result<faer::Mat<f64>, String> {
    if lhs.ncols() != rhs.nrows() {
        return Err(format!(
            "cannot matrix-multiply shapes ({}, {}) and ({}, {})",
            lhs.nrows(),
            lhs.ncols(),
            rhs.nrows(),
            rhs.ncols(),
        ));
    }

    let rows = lhs.nrows();

    let cols = rhs.ncols();

    let inner = lhs.ncols();

    let mut result = faer::Mat::<f64>::zeros(rows, cols);

    let work = rows.saturating_mul(cols).saturating_mul(inner);

    let par = if should_parallelize(work) {
        faer::Par::rayon(0)
    } else {
        faer::Par::Seq
    };

    faer::linalg::matmul::matmul(&mut result, faer::Accum::Replace, lhs, rhs, 1.0, par);

    Ok(result)
}

pub fn matrix_elementwise_mul(lhs: &Mat<f64>, rhs: &Mat<f64>) -> Mat<f64> {
    let rows = lhs.nrows();

    let cols = lhs.ncols();

    let mut result = Mat::<f64>::zeros(rows, cols);

    faer::zip!(&mut result, lhs, rhs,).for_each(|faer::unzip!(result, lhs, rhs)| {
        *result = lhs * rhs;
    });

    result
}

pub fn vector_matrix_mul(
    vector: &faer::Col<f64>,
    matrix: &faer::Mat<f64>,
) -> Result<faer::Col<f64>, String> {
    if vector.nrows() != matrix.nrows() {
        return Err(format!(
            "vector-matrix multiplication dimension mismatch: vector length {}, matrix shape ({}, {})",
            vector.nrows(),
            matrix.nrows(),
            matrix.ncols(),
        ));
    }

    let transposed = matrix.transpose();

    let work = matrix.nrows().saturating_mul(matrix.ncols());

    let par = if should_parallelize(work) {
        Par::rayon(0)
    } else {
        Par::Seq
    };

    let mut result = faer::Col::<f64>::zeros(matrix.ncols());

    faer::linalg::matmul::matmul(
        result.as_mat_mut(),
        Accum::Replace,
        &transposed,
        vector.as_mat(),
        1.0,
        par,
    );

    Ok(result)
}

pub fn matrix_vector_mul(
    matrix: &Mat<f64>,
    vector: &faer::Col<f64>,
) -> Result<faer::Col<f64>, String> {
    if matrix.ncols() != vector.nrows() {
        return Err(format!(
            "matrix-vector multiplication dimension mismatch: matrix shape ({}, {}), vector length {}",
            matrix.nrows(),
            matrix.ncols(),
            vector.nrows(),
        ));
    }

    let mut result = faer::Col::<f64>::zeros(matrix.nrows());

    let work = matrix.nrows().saturating_mul(matrix.ncols());

    let par = if should_parallelize(work) {
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

pub fn matrix_determinant(matrix: &faer::Mat<f64>) -> Result<f64, String> {
    if matrix.nrows() != matrix.ncols() {
        return Err("determinant requires a square matrix".into());
    }

    let n = matrix.nrows();

    if n == 0 {
        return Err("determinant requires a non-empty matrix".into());
    }

    let lu = matrix.partial_piv_lu();

    let u = lu.U();

    let mut determinant = 1.0;

    for i in 0..n {
        determinant *= u[(i, i)];
    }

    /*
     * faer returns P such that:
     *
     *     P A = L U
     *
     * Therefore:
     *
     *     det(A) = det(P) * det(U)
     *
     * because det(P) = det(P^-1) = ±1.
     *
     * The permutation sign is:
     *
     *     (-1)^(n - number_of_cycles)
     */
    let permutation = lu.P();

    let (forward, _) = permutation.arrays();

    let mut visited = vec![false; n];

    let mut cycles = 0usize;

    for start in 0..n {
        if visited[start] {
            continue;
        }

        cycles += 1;

        let mut current = start;

        while !visited[current] {
            visited[current] = true;

            current = forward[current];
        }
    }

    if (n - cycles) % 2 == 1 {
        determinant = -determinant;
    }

    Ok(determinant)
}

pub fn matrix_solve(lhs: &faer::Mat<f64>, rhs: &faer::Mat<f64>) -> Result<faer::Mat<f64>, String> {
    if lhs.nrows() != lhs.ncols() {
        return Err("solve requires a square coefficient matrix".into());
    }

    if lhs.nrows() != rhs.nrows() {
        return Err(format!(
            "solve dimension mismatch: A has {} rows, rhs has {} rows",
            lhs.nrows(),
            rhs.nrows(),
        ));
    }

    let lu = lhs.partial_piv_lu();

    Ok(lu.solve(rhs))
}

pub fn matrix_solve_lstsq(
    lhs: &faer::Mat<f64>,
    rhs: &faer::Mat<f64>,
) -> Result<faer::Mat<f64>, String> {
    if lhs.nrows() != rhs.nrows() {
        return Err(format!(
            "least-squares dimension mismatch: A has {} rows, rhs has {} rows",
            lhs.nrows(),
            rhs.nrows(),
        ));
    }

    if lhs.nrows() == 0 || lhs.ncols() == 0 {
        return Err("least-squares solve requires a non-empty matrix".into());
    }

    let qr = lhs.col_piv_qr();

    Ok(qr.solve_lstsq(rhs))
}
