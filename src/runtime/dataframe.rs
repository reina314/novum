use super::{
    SeriesRef,
    Value,
};

use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type DataFrameRef =
    Rc<DataFrame>;

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
                "DataFrame must have at least one column"
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
            if index.contains_key(
                column.name()
            ) {
                return Err(format!(
                    "duplicate column name '{}'",
                    column.name()
                ));
            }

            index.insert(
                column.name().to_owned(),
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

    pub fn column(
        &self,
        name: &str,
    ) -> Option<SeriesRef> {
        self.index
            .get(name)
            .map(|index| {
                self.columns[*index].clone()
            })
    }

    pub fn columns(
        &self,
    ) -> Vec<String> {
        self.columns
            .iter()
            .map(|x| x.name().to_owned())
            .collect()
    }

    pub fn to_matrix(
        &self,
        names: &[String],
    ) -> Result<crate::runtime::Matrix, String> {
        if names.is_empty() {
            return Err(
                "to_matrix() requires at least one column"
                    .into()
            );
        }

        let mut columns = Vec::new();

        for name in names {
            let column =
                self.column(name)
                    .ok_or_else(|| {
                        format!(
                            "unknown DataFrame column '{}'",
                            name
                        )
                    })?;

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

                    _ => {
                        return Err(format!(
                            "column '{}' is not numeric",
                            name
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

            for column in
                &columns
            {
                row.push(
                    column[r]
                );
            }

            rows.push(row);
        }

        crate::runtime::Matrix::from_rows(
            rows
        )
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