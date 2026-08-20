use super::{
    Matrix,
    SeriesRef,
    Value,
};

use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type DataFrameRef = Rc<DataFrame>;

#[derive(Clone)]
pub struct DataFrame {
    columns: Vec<SeriesRef>,
    index: HashMap<String, usize>,
    nrows: usize,
}

impl DataFrame {
    pub fn from_series(
        columns: Vec<SeriesRef>,
    ) -> Result<Self, String> {
        if columns.is_empty() {
            return Err(
                "DataFrame must contain at least one column"
                    .into()
            );
        }

        let nrows =
            columns[0].len();

        if columns.iter().any(
            |column| column.len() != nrows
        ) {
            return Err(
                "all DataFrame columns must have the same length"
                    .into()
            );
        }

        let mut index =
            HashMap::new();

        for (i, column) in
            columns.iter().enumerate()
        {
            let name =
                column.name();

            if index.contains_key(name) {
                return Err(format!(
                    "duplicate DataFrame column '{}'",
                    name
                ));
            }

            index.insert(
                name.to_owned(),
                i,
            );
        }

        Ok(Self {
            columns,
            index,
            nrows,
        })
    }

    pub fn nrows(&self) -> usize {
        self.nrows
    }

    pub fn ncols(&self) -> usize {
        self.columns.len()
    }

    pub fn columns(&self) -> Vec<String> {
        self.columns
            .iter()
            .map(|column| {
                column.name().to_owned()
            })
            .collect()
    }

    pub fn column(
        &self,
        name: &str,
    ) -> Option<SeriesRef> {
        self.index
            .get(name)
            .map(|&index| {
                self.columns[index].clone()
            })
    }

    pub fn select(
        &self,
        names: &[String],
    ) -> Result<Self, String> {
        if names.is_empty() {
            return Err(
                "select() requires at least one column"
                    .into()
            );
        }

        let mut columns =
            Vec::with_capacity(
                names.len()
            );

        for name in names {
            let column =
                self.column(name)
                    .ok_or_else(|| {
                        format!(
                            "unknown DataFrame column '{}'",
                            name
                        )
                    })?;

            columns.push(column);
        }

        Self::from_series(columns)
    }

    pub fn to_matrix(
        &self,
    ) -> Result<Matrix, String> {
        if self.nrows == 0 {
            return Err(
                "cannot convert empty DataFrame to Matrix"
                    .into()
            );
        }

        let mut columns =
            Vec::<Vec<f64>>::with_capacity(
                self.columns.len()
            );

        for column in &self.columns {
            let mut values =
                Vec::with_capacity(
                    self.nrows
                );

            for value in column.data() {
                match value {
                    Value::Int(v) =>
                        values.push(*v as f64),

                    Value::Float(v) =>
                        values.push(*v),

                    Value::Null => {
                        return Err(format!(
                            "column '{}' contains Null",
                            column.name()
                        ));
                    }

                    other => {
                        return Err(format!(
                            "column '{}' is not numeric; found {}",
                            column.name(),
                            other.type_name()
                        ));
                    }
                }
            }

            columns.push(values);
        }

        let mut rows =
            Vec::with_capacity(
                self.nrows
            );

        for r in 0..self.nrows {
            let mut row =
                Vec::with_capacity(
                    columns.len()
                );

            for column in &columns {
                row.push(column[r]);
            }

            rows.push(row);
        }

        Matrix::from_rows(rows)
    }
}

impl fmt::Debug for DataFrame {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("DataFrame")
            .field(
                "columns",
                &self.columns(),
            )
            .field(
                "nrows",
                &self.nrows,
            )
            .finish()
    }
}

impl fmt::Display for DataFrame {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        writeln!(
            f,
            "DataFrame: {} rows × {} columns",
            self.nrows(),
            self.ncols()
        )?;

        for (i, name) in
            self.columns().iter().enumerate()
        {
            if i > 0 {
                write!(f, " | ")?;
            }

            write!(f, "{name}")?;
        }

        writeln!(f)
    }
}