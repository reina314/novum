use super::Matrix;

use std::{
    fmt,
    rc::Rc,
    cell::RefCell,
};

pub type VectorRef = Rc<RefCell<Vector>>;

#[derive(Clone)]
pub struct Vector {
    data: Vec<f64>,
}

impl Vector {
    pub fn new(
        data: Vec<f64>,
    ) -> Self {
        Self { data }
    }

    pub fn from_slice(
        data: &[f64],
    ) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    pub fn shape(
        &self,
    ) -> (usize, usize) {
        (self.len(), 1)
    }

    pub fn len(
        &self,
    ) -> usize {
        self.data.len()
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Option<f64> {
        self.data.get(index).copied()
    }

    pub fn as_slice(
        &self,
    ) -> &[f64] {
        &self.data
    }

    pub fn into_vec(
        self,
    ) -> Vec<f64> {
        self.data
    }

    pub fn norm(
        &self,
    ) -> f64 {
        self.data
            .iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt()
    }

    pub fn dot(
        &self,
        other: &Self,
    ) -> Result<f64, String> {
        if self.len() != other.len() {
            return Err(format!(
                "dot product requires vectors of equal length, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            self.data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| a * b)
                .sum()
        )
    }

    pub fn add(
        &self,
        other: &Self,
    ) -> Result<Self, String> {
        if self.len() != other.len() {
            return Err(format!(
                "vector addition requires equal lengths, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            Self::new(
                self.data
                    .iter()
                    .zip(other.data.iter())
                    .map(|(a, b)| a + b)
                    .collect()
            )
        )
    }

    pub fn sub(
        &self,
        other: &Self,
    ) -> Result<Self, String> {
        if self.len() != other.len() {
            return Err(format!(
                "vector subtraction requires equal lengths, got {} and {}",
                self.len(),
                other.len(),
            ));
        }

        Ok(
            Self::new(
                self.data
                    .iter()
                    .zip(other.data.iter())
                    .map(|(a, b)| a - b)
                    .collect()
            )
        )
    }

    pub fn scale(
        &self,
        scalar: f64,
    ) -> Self {
        Self::new(
            self.data
                .iter()
                .map(|x| x * scalar)
                .collect()
        )
    }

    pub fn to_column_matrix(
        &self,
    ) -> Matrix {
        Matrix::from_vec(
            self.len(),
            1,
            self.data.clone(),
        )
        .expect("vector has valid matrix shape")
    }

    pub fn to_row_matrix(
        &self,
    ) -> Matrix {
        Matrix::from_vec(
            1,
            self.len(),
            self.data.clone(),
        )
        .expect("vector has valid matrix shape")
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

        let mut data =
            Vec::with_capacity(
                matrix.rows()
            );

        for row in 0..matrix.rows() {
            let value =
                matrix
                    .get(row, 0)
                    .ok_or_else(|| {
                        "matrix index out of bounds".to_owned()
                    })?;

            data.push(value);
        }

        Ok(Self::new(data))
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

        let mut data =
            Vec::with_capacity(
                matrix.cols()
            );

        for col in 0..matrix.cols() {
            let value =
                matrix
                    .get(0, col)
                    .ok_or_else(|| {
                        "matrix index out of bounds".to_owned()
                    })?;

            data.push(value);
        }

        Ok(Self::new(data))
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "(")?;

        for (i, value) in self.as_slice().iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            write!(f, "{value}")?;
        }

        write!(f, ")")
    }
}

impl fmt::Debug for Vector {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("Vector")
            .field(&self.data)
            .finish()
    }
}