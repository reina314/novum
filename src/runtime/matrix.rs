use std::{
    cell::RefCell,
    fmt,
    rc::Rc,
};

use faer::{
    linalg::solvers::DenseSolveCore,
    Mat,
};

pub type MatrixRef =
    Rc<RefCell<Matrix>>;

#[derive(Clone, Debug)]
pub struct Matrix {
    data: Mat<f64>,
}

impl Matrix {
    pub fn new(
        rows: usize,
        cols: usize,
        data: Vec<f64>,
    ) -> Result<Self, String> {
        if rows == 0
            || cols == 0
        {
            return Err(
                "matrix dimensions must be non-zero"
                    .into()
            );
        }

        let expected =
            rows.checked_mul(cols)
                .ok_or_else(|| {
                    "matrix dimensions overflow"
                        .to_string()
                })?;

        if data.len()
            != expected
        {
            return Err(format!(
                "invalid matrix data length: expected {}, got {}",
                expected,
                data.len()
            ));
        }

        let matrix =
            Mat::from_fn(
                rows,
                cols,
                |row, col| {
                    data[
                        row * cols + col
                    ]
                },
            );

        Ok(
            Self {
                data: matrix,
            }
        )
    }

    pub fn from_rows(
        rows: Vec<Vec<f64>>,
    ) -> Result<Self, String> {
        if rows.is_empty() {
            return Err(
                "matrix must not be empty"
                    .into()
            );
        }

        let cols =
            rows[0].len();

        if cols == 0 {
            return Err(
                "matrix rows must not be empty"
                    .into()
            );
        }

        if rows.iter()
            .any(|row| {
                row.len() != cols
            })
        {
            return Err(
                "matrix must be rectangular"
                    .into()
            );
        }

        let nrows =
            rows.len();

        let data =
            Mat::from_fn(
                nrows,
                cols,
                |row, col| {
                    rows[row][col]
                },
            );

        Ok(
            Self {
                data,
            }
        )
    }

    pub fn from_vec(
        rows: usize,
        cols: usize,
        data: Vec<f64>,
    ) -> Result<Self, String> {
        let expected =
            rows.checked_mul(cols)
                .ok_or_else(|| {
                    "matrix dimensions overflow"
                        .to_string()
                })?;

        if expected != data.len() {
            return Err(format!(
                "matrix data length mismatch: shape ({}, {}) requires {} elements, got {}",
                rows,
                cols,
                expected,
                data.len(),
            ));
        }

        let matrix =
            Mat::from_fn(
                rows,
                cols,
                |row, col| {
                    data[
                        row * cols + col
                    ]
                },
            );

        Ok(
            Self {
                data: matrix,
            }
        )
    }

    pub(crate) fn from_faer(
        data: Mat<f64>,
    ) -> Self {
        Self {
            data,
        }
    }

    pub(crate) fn from_column_vector(
        vector: &faer::Col<f64>,
    ) -> Self {
        Self {
            data:
                Mat::from_fn(
                    vector.nrows(),
                    1,
                    |row, _| {
                        vector[row]
                    },
                ),
        }
    }

    pub(crate) fn from_row_vector(
        vector: &faer::Col<f64>,
    ) -> Self {
        Self {
            data:
                Mat::from_fn(
                    1,
                    vector.nrows(),
                    |_, col| {
                        vector[col]
                    },
                ),
        }
    }

    pub(crate) fn as_faer(
        &self,
    ) -> &Mat<f64> {
        &self.data
    }

    pub fn rows(
        &self,
    ) -> usize {
        self.data.nrows()
    }

    pub fn cols(
        &self,
    ) -> usize {
        self.data.ncols()
    }

    pub fn shape(
        &self,
    ) -> (usize, usize) {
        (
            self.rows(),
            self.cols(),
        )
    }

    pub fn get(
        &self,
        row: usize,
        col: usize,
    ) -> Option<f64> {
        if row >= self.rows()
            || col >= self.cols()
        {
            return None;
        }

        Some(
            self.data[
                (row, col)
            ]
        )
    }

    pub fn set(
        &mut self,
        row: usize,
        col: usize,
        value: f64,
    ) -> Result<(), String> {
        if row >= self.rows()
            || col >= self.cols()
        {
            return Err(format!(
                "matrix index out of bounds: ({}, {})",
                row,
                col
            ));
        }

        self.data[
            (row, col)
        ] = value;

        Ok(())
    }

    pub fn trace(
        &self,
    ) -> Result<f64, String> {
        if self.rows()
            != self.cols()
        {
            return Err(format!(
                "trace requires a square matrix, got shape ({}, {})",
                self.rows(),
                self.cols(),
            ));
        }

        let mut result =
            0.0;

        for i in 0..self.rows() {
            result +=
                self.data[
                    (i, i)
                ];
        }

        Ok(result)
    }

    pub fn approx_eq(
        &self,
        rhs: &Self,
        abs_tol: f64,
        rel_tol: f64,
    ) -> bool {
        if self.shape()
            != rhs.shape()
        {
            return false;
        }

        for row in 0..self.rows() {
            for col in 0..self.cols() {
                let a =
                    self.data[
                        (row, col)
                    ];

                let b =
                    rhs.data[
                        (row, col)
                    ];

                let diff =
                    (a - b).abs();

                if diff >
                    abs_tol
                        + rel_tol * b.abs()
                {
                    return false;
                }
            }
        }

        true
    }

    pub fn slice(
        &self,
        row_start: usize,
        row_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Result<Self, String> {
        if row_start > row_end
            || col_start > col_end
        {
            return Err(
                "invalid matrix slice range"
                    .into()
            );
        }

        if row_end > self.rows()
            || col_end > self.cols()
        {
            return Err(format!(
                "matrix slice out of bounds: rows {}..{}, cols {}..{} for shape {:?}",
                row_start,
                row_end,
                col_start,
                col_end,
                self.shape()
            ));
        }

        if row_start == row_end
            || col_start == col_end
        {
            return Err(
                "matrix slice must not be empty"
                    .into()
            );
        }

        let rows =
            row_end - row_start;

        let cols =
            col_end - col_start;

        let data =
            Mat::from_fn(
                rows,
                cols,
                |row, col| {
                    self.data[
                        (
                            row_start + row,
                            col_start + col,
                        )
                    ]
                },
            );

        Ok(
            Self {
                data,
            }
        )
    }

    pub fn add(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape()
            != rhs.shape()
        {
            return Err(format!(
                "cannot add matrices with shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::matrix_add(
                        &self.data,
                        &rhs.data,
                    )?,
            }
        )
    }

    pub fn sub(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape()
            != rhs.shape()
        {
            return Err(format!(
                "cannot subtract matrices with shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::matrix_sub(
                        &self.data,
                        &rhs.data,
                    )?,
            }
        )
    }

    pub fn elementwise_mul(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape()
            != rhs.shape()
        {
            return Err(format!(
                "element-wise multiplication requires equal shapes: {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::matrix_elementwise_mul(
                        &self.data,
                        &rhs.data,
                    ),
            }
        )
    }

    pub fn scalar_mul(
        &self,
        scalar: f64,
    ) -> Self {
        Self {
            data:
                crate::runtime::numeric::matrix_scale(
                    &self.data,
                    scalar,
                ),
        }
    }

    pub fn matmul(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.cols()
            != rhs.rows()
        {
            return Err(format!(
                "cannot matrix-multiply shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::matrix_matmul(
                        &self.data,
                        &rhs.data,
                    )?,
            }
        )
    }

    pub fn transpose(
        &self,
    ) -> Self {
        Self {
            data:
                self.data
                    .transpose()
                    .to_owned(),
        }
    }

    pub fn determinant(
        &self,
    ) -> Result<f64, String> {
        if self.rows()
            != self.cols()
        {
            return Err(
                "determinant requires a square matrix"
                    .into()
            );
        }

        crate::runtime::numeric::matrix_determinant(
            &self.data
        )
    }

    pub fn inverse(
        &self,
    ) -> Result<Self, String> {
        if self.rows()
            != self.cols()
        {
            return Err(
                "inverse requires a square matrix"
                    .into()
            );
        }

        let lu =
            self.data
                .partial_piv_lu();

        Ok(
            Self {
                data:
                    lu.inverse(),
            }
        )
    }

    pub fn to_rows(
        &self,
    ) -> Vec<Vec<f64>> {
        (0..self.rows())
            .map(|row| {
                (0..self.cols())
                    .map(|col| {
                        self.data[
                            (row, col)
                        ]
                    })
                    .collect()
            })
            .collect()
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let rows =
            self.rows();

        let cols =
            self.cols();

        if rows == 0 {
            return write!(
                f,
                "()"
            );
        }

        let mut values =
            Vec::with_capacity(rows);

        for row in 0..rows {
            let mut values_row =
                Vec::with_capacity(cols);

            for col in 0..cols {
                values_row.push(
                    self.data[
                        (row, col)
                    ]
                    .to_string()
                );
            }

            values.push(
                values_row
            );
        }

        let mut widths =
            vec![0usize; cols];

        for col in 0..cols {
            for row in 0..rows {
                widths[col] =
                    widths[col]
                        .max(
                            values[row][col]
                                .len()
                        );
            }
        }

        writeln!(
            f,
            "("
        )?;

        for row in 0..rows {
            write!(
                f,
                "    [ "
            )?;

            for col in 0..cols {
                if col > 0 {
                    write!(
                        f,
                        ", "
                    )?;
                }

                write!(
                    f,
                    "{:>width$}",
                    values[row][col],
                    width = widths[col],
                )?;
            }

            if row + 1 == rows {
                writeln!(
                    f,
                    " ]"
                )?;
            } else {
                writeln!(
                    f,
                    " ],"
                )?;
            }
        }

        write!(
            f,
            ")"
        )
    }
}