use super::Matrix;

use std::{
    cell::RefCell,
    fmt,
    rc::Rc,
};

pub type VectorRef =
    Rc<RefCell<Vector>>;

#[derive(Clone)]
pub struct Vector {
    data: faer::Col<f64>,
}

impl Vector {
    pub fn new(
        data: Vec<f64>,
    ) -> Self {
        Self {
            data:
                faer::Col::from_fn(
                    data.len(),
                    |i| data[i],
                ),
        }
    }

    pub fn from_slice(
        data: &[f64],
    ) -> Self {
        Self::new(
            data.to_vec()
        )
    }

    pub fn shape(
        &self,
    ) -> (usize, usize) {
        (
            self.len(),
            1,
        )
    }

    pub fn len(
        &self,
    ) -> usize {
        self.data.nrows()
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Option<f64> {
        if index >= self.data.nrows() {
            return None;
        }

        Some(
            self.data[index]
        )
    }

    pub fn as_slice(
        &self,
    ) -> &[f64] {
        self.data
            .try_as_col_major()
            .expect(
                "Vector must be contiguous",
            )
            .as_slice()
    }

    pub fn into_vec(
        self,
    ) -> Vec<f64> {
        self.data
            .iter()
            .copied()
            .collect()
    }

    pub fn norm(
        &self,
    ) -> f64 {
        crate::runtime::numeric::vector_norm(
            &self.data
        )
    }

    pub fn dot(
        &self,
        other: &Self,
    ) -> Result<f64, String> {
        if self.len()
            != other.len()
        {
            return Err(format!(
                "dot product requires vectors of equal length, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            crate::runtime::numeric::vector_dot(
                &self.data,
                &other.data,
            )
        )
    }

    pub fn add(
        &self,
        other: &Self,
    ) -> Result<Self, String> {
        if self.len()
            != other.len()
        {
            return Err(format!(
                "vector addition requires equal lengths, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::vector_add(
                        &self.data,
                        &other.data,
                    ),
            }
        )
    }

    pub fn sub(
        &self,
        other: &Self,
    ) -> Result<Self, String> {
        if self.len()
            != other.len()
        {
            return Err(format!(
                "vector subtraction requires equal lengths, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            Self {
                data:
                    crate::runtime::numeric::vector_sub(
                        &self.data,
                        &other.data,
                    ),
            }
        )
    }

    pub fn scale(
        &self,
        scalar: f64,
    ) -> Self {
        Self {
            data:
                crate::runtime::numeric::vector_scale(
                    &self.data,
                    scalar,
                ),
        }
    }

    pub fn to_column_matrix(
        &self,
    ) -> Matrix {
        Matrix::from_column_vector(
            &self.data
        )
    }

    pub fn to_row_matrix(
        &self,
    ) -> Matrix {
        Matrix::from_row_vector(
            &self.data
        )
    }

    pub fn from_matrix_column(
        matrix: &Matrix,
    ) -> Result<Self, String> {
        if matrix.cols() != 1 {
            return Err(format!(
                "expected a column matrix, got shape ({}, {})",
                matrix.rows(),
                matrix.cols(),
            ));
        }

        Ok(
            Self {
                data:
                    faer::Col::from_fn(
                        matrix.rows(),
                        |row| {
                            matrix
                                .as_faer()
                                [(row, 0)]
                        },
                    ),
            }
        )
    }

    pub fn from_matrix_row(
        matrix: &Matrix,
    ) -> Result<Self, String> {
        if matrix.rows() != 1 {
            return Err(format!(
                "expected a row matrix, got shape ({}, {})",
                matrix.rows(),
                matrix.cols(),
            ));
        }

        Ok(
            Self {
                data:
                    faer::Col::from_fn(
                        matrix.cols(),
                        |col| {
                            matrix
                                .as_faer()
                                [(0, col)]
                        },
                    ),
            }
        )
    }

    pub(crate) fn as_faer(
        &self,
    ) -> &faer::Col<f64> {
        &self.data
    }

    pub(crate) fn from_faer(
        data: faer::Col<f64>,
    ) -> Self {
        Self {
            data,
        }
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "(")?;

        for (
            i,
            value,
        ) in self.data.iter().enumerate()
        {
            if i > 0 {
                write!(
                    f,
                    ", "
                )?;
            }

            write!(
                f,
                "{value}"
            )?;
        }

        write!(f, ")")
    }
}

impl fmt::Debug for Vector {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple(
            "Vector"
        )
        .field(
            &self
                .data
                .iter()
                .copied()
                .collect::<Vec<_>>()
        )
        .finish()
    }
}