use std::{
    fmt,
    rc::Rc,
    cell::RefCell,
};

pub type MatrixRef =
    Rc<RefCell<Matrix>>;

#[derive(Clone)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    pub fn new(
        rows: usize,
        cols: usize,
        data: Vec<f64>,
    ) -> Result<Self, String> {
        if rows == 0 || cols == 0 {
            return Err(
                "matrix dimensions must be non-zero"
                    .into()
            );
        }

        if data.len() != rows * cols {
            return Err(format!(
                "invalid matrix data length: expected {}, got {}",
                rows * cols,
                data.len()
            ));
        }

        Ok(Self {
            rows,
            cols,
            data,
        })
    }

    pub fn from_rows(
        rows: Vec<Vec<f64>>,
    ) -> Result<Self, String> {
        if rows.is_empty() {
            return Err(
                "matrix must not be empty".into()
            );
        }

        let cols = rows[0].len();

        if cols == 0 {
            return Err(
                "matrix rows must not be empty"
                    .into()
            );
        }

        if rows.iter()
            .any(|row| row.len() != cols)
        {
            return Err(
                "matrix must be rectangular"
                    .into()
            );
        }

        let data: Vec<f64> =
            rows.into_iter()
                .flatten()
                .collect();

        Self::new(
            data.len() / cols,
            cols,
            data,
        )
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
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
                "invalid matrix slice range".into()
            );
        }

        if row_end > self.rows
            || col_end > self.cols
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
                "matrix slice must not be empty".into()
            );
        }

        let rows =
            row_end - row_start;

        let cols =
            col_end - col_start;

        let mut data =
            Vec::with_capacity(rows * cols);

        for r in row_start..row_end {
            for c in col_start..col_end {
                data.push(
                    self.data[
                        r * self.cols + c
                    ]
                );
            }
        }

        Ok(Self {
            rows,
            cols,
            data,
        })
    }

    pub fn get(
        &self,
        row: usize,
        col: usize,
    ) -> Option<f64> {
        if row >= self.rows
            || col >= self.cols
        {
            return None;
        }

        Some(
            self.data[
                row * self.cols + col
            ]
        )
    }

    pub fn set(
        &mut self,
        row: usize,
        col: usize,
        value: f64,
    ) -> Result<(), String> {
        if row >= self.rows
            || col >= self.cols
        {
            return Err(format!(
                "matrix index out of bounds: ({}, {})",
                row,
                col
            ));
        }

        self.data[
            row * self.cols + col
        ] = value;

        Ok(())
    }

    pub fn approx_eq(
        &self,
        rhs: &Self,
        abs_tol: f64,
        rel_tol: f64,
    ) -> bool {
        if self.shape() != rhs.shape() {
            return false;
        }

        self.data
            .iter()
            .zip(rhs.data.iter())
            .all(|(a, b)| {
                let diff = (a - b).abs();

                diff <=
                    abs_tol
                    + rel_tol * b.abs()
            })
    }

    pub fn add(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape() != rhs.shape() {
            return Err(format!(
                "cannot add matrices with shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        let data =
            self.data
                .iter()
                .zip(rhs.data.iter())
                .map(|(a, b)| a + b)
                .collect();

        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    pub fn sub(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape() != rhs.shape() {
            return Err(format!(
                "cannot subtract matrices with shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        let data =
            self.data
                .iter()
                .zip(rhs.data.iter())
                .map(|(a, b)| a - b)
                .collect();

        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    pub fn elementwise_mul(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.shape() != rhs.shape() {
            return Err(format!(
                "element-wise multiplication requires equal shapes: {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        let data =
            self.data
                .iter()
                .zip(rhs.data.iter())
                .map(|(a, b)| a * b)
                .collect();

        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    pub fn scalar_mul(
        &self,
        scalar: f64,
    ) -> Self {
        Self {
            rows: self.rows,
            cols: self.cols,

            data: self.data
                .iter()
                .map(|x| x * scalar)
                .collect(),
        }
    }

    pub fn matmul(
        &self,
        rhs: &Self,
    ) -> Result<Self, String> {
        if self.cols != rhs.rows {
            return Err(format!(
                "cannot matrix-multiply shapes {:?} and {:?}",
                self.shape(),
                rhs.shape()
            ));
        }

        let mut data =
            vec![
                0.0;
                self.rows * rhs.cols
            ];

        for i in 0..self.rows {
            for k in 0..self.cols {
                let a =
                    self.data[
                        i * self.cols + k
                    ];

                for j in 0..rhs.cols {
                    data[
                        i * rhs.cols + j
                    ] += a * rhs.data[
                        k * rhs.cols + j
                    ];
                }
            }
        }

        Ok(Self {
            rows: self.rows,
            cols: rhs.cols,
            data,
        })
    }

    pub fn transpose(&self) -> Self {
        let mut data =
            vec![
                0.0;
                self.rows * self.cols
            ];

        for r in 0..self.rows {
            for c in 0..self.cols {
                data[
                    c * self.rows + r
                ] =
                    self.data[
                        r * self.cols + c
                    ];
            }
        }

        Self {
            rows: self.cols,
            cols: self.rows,
            data,
        }
    }

    pub fn determinant(
        &self,
    ) -> Result<f64, String> {
        if self.rows != self.cols {
            return Err(
                "determinant requires a square matrix"
                    .into()
            );
        }

        let n = self.rows;
        let mut a = self.data.clone();

        let mut determinant = 1.0;

        for i in 0..n {
            let mut pivot = i;

            for r in (i + 1)..n {
                if a[
                    r * n + i
                ].abs()
                    > a[
                        pivot * n + i
                    ].abs()
                {
                    pivot = r;
                }
            }

            let pivot_value =
                a[pivot * n + i];

            if pivot_value.abs() < 1e-12 {
                return Ok(0.0);
            }

            if pivot != i {
                for c in 0..n {
                    a.swap(
                        i * n + c,
                        pivot * n + c,
                    );
                }

                determinant = -determinant;
            }

            let pivot_value =
                a[i * n + i];

            determinant *= pivot_value;

            for r in (i + 1)..n {
                let factor =
                    a[r * n + i]
                    / pivot_value;

                for c in i..n {
                    a[r * n + c]
                        -= factor
                            * a[
                                i * n + c
                            ];
                }
            }
        }

        Ok(determinant)
    }

    pub fn inverse(
        &self,
    ) -> Result<Self, String> {
        if self.rows != self.cols {
            return Err(
                "inverse requires a square matrix"
                    .into()
            );
        }

        let n = self.rows;
        let width = n * 2;

        let mut a =
            vec![0.0; n * width];

        for r in 0..n {
            for c in 0..n {
                a[r * width + c] =
                    self.data[
                        r * n + c
                    ];

                a[
                    r * width + n + c
                ] =
                    if r == c {
                        1.0
                    } else {
                        0.0
                    };
            }
        }

        for i in 0..n {
            let mut pivot = i;

            for r in (i + 1)..n {
                if a[
                    r * width + i
                ].abs()
                    > a[
                        pivot * width + i
                    ].abs()
                {
                    pivot = r;
                }
            }

            if a[
                pivot * width + i
            ].abs() < 1e-12 {
                return Err(
                    "matrix is singular and cannot be inverted"
                        .into()
                );
            }

            if pivot != i {
                for c in 0..width {
                    a.swap(
                        i * width + c,
                        pivot * width + c,
                    );
                }
            }

            let pivot_value =
                a[i * width + i];

            for c in 0..width {
                a[i * width + c]
                    /= pivot_value;
            }

            for r in 0..n {
                if r == i {
                    continue;
                }

                let factor =
                    a[r * width + i];

                for c in 0..width {
                    a[r * width + c]
                        -= factor
                            * a[
                                i * width + c
                            ];
                }
            }
        }

        let mut data =
            vec![0.0; n * n];

        for r in 0..n {
            for c in 0..n {
                data[
                    r * n + c
                ] =
                    a[
                        r * width + n + c
                    ];
            }
        }

        Ok(Self {
            rows: n,
            cols: n,
            data,
        })
    }

    pub fn to_rows(
        &self,
    ) -> Vec<Vec<f64>> {
        (0..self.rows)
            .map(|r| {
                (0..self.cols)
                    .map(|c| {
                        self.data[
                            r * self.cols + c
                        ]
                    })
                    .collect()
            })
            .collect()
    }
}

impl fmt::Debug for Matrix {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Matrix")
            .field("shape", &self.shape())
            .field("data", &self.to_rows())
            .finish()
    }
}