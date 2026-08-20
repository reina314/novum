use super::{
    Matrix,
    SeriesRef,
    Object,
    ObjectRef,
    Value,
};

use std::{
    collections::HashMap,
    fmt,
    cell::RefCell,
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

    pub fn row(
        &self,
        index: usize,
    ) -> Option<ObjectRef> {
        if index >= self.nrows {
            return None;
        }

        let mut object =
            Object::new();

        for column in &self.columns {
            let value =
                column.get(index)?;

            object.set_field(
                column.name().to_owned(),
                value,
            );
        }

        Some(
            Rc::new(
                RefCell::new(object)
            )
        )
    }

    pub fn take_rows(
        &self,
        indices: &[usize],
    ) -> Result<Self, String> {
        let mut columns =
            Vec::with_capacity(
                self.columns.len()
            );

        for column in &self.columns {
            let mut values =
                Vec::with_capacity(
                    indices.len()
                );

            for &index in indices {
                let value =
                    column
                        .get(index)
                        .ok_or_else(|| {
                            format!(
                                "row index out of bounds: {}",
                                index
                            )
                        })?;

                values.push(value);
            }

            columns.push(
                Rc::new(
                    crate::runtime::Series::new(
                        column.name().to_owned(),
                        values,
                    )
                )
            );
        }

        Self::from_series(columns)
    }

    pub fn filter_rows(
        &self,
        keep: &[bool],
    ) -> Result<Self, String> {
        if keep.len() != self.nrows {
            return Err(
                "filter mask length must equal DataFrame row count"
                    .into()
            );
        }

        let indices =
            keep.iter()
                .enumerate()
                .filter_map(
                    |(index, keep)| {
                        if *keep {
                            Some(index)
                        } else {
                            None
                        }
                    }
                )
                .collect::<Vec<_>>();

        self.take_rows(&indices)
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

    pub fn head(
        &self,
        n: usize,
    ) -> Result<Self, String> {
        let end =
            n.min(self.nrows);

        let indices =
            (0..end)
                .collect::<Vec<_>>();

        self.take_rows(&indices)
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        const MAX_ROWS: usize = 10;

        let nrows = self.nrows();
        let ncols = self.ncols();
        let names = self.columns();

        writeln!(
            f,
            "DataFrame ({} rows x {} columns)",
            nrows,
            ncols
        )?;

        #[derive(Clone, Copy)]
        enum Align {
            Left,
            Right,
            Center,
        }

        fn cell_text(value: Option<Value>) -> (String, Align) {
            match value {
                None => (
                    "null".to_string(),
                    Align::Center,
                ),

                Some(value) => {
                    let align = match &value {
                        Value::Int(_)
                        | Value::Float(_) =>
                            Align::Right,

                        Value::Bool(_) =>
                            Align::Center,

                        Value::Str(_) =>
                            Align::Left,

                        _ =>
                            Align::Left,
                    };

                    (value.to_string(), align)
                }
            }
        }

        // --------------------------------------------
        // Rows
        // --------------------------------------------

        let rows: Vec<Option<usize>> =
            if nrows <= MAX_ROWS {
                (0..nrows).map(Some).collect()
            } else {
                let head = MAX_ROWS / 2;
                let tail = MAX_ROWS - head;

                let mut rows = Vec::with_capacity(MAX_ROWS + 1);

                rows.extend((0..head).map(Some));
                rows.push(None);
                rows.extend(
                    (nrows - tail..nrows).map(Some)
                );

                rows
            };

        // --------------------------------------------
        // Cells
        // --------------------------------------------

        let cells: Vec<Vec<(String, Align)>> = rows
            .iter()
            .map(|row| {
                match row {
                    Some(row) => {
                        (0..ncols)
                            .map(|col| {
                                cell_text(
                                    self.columns[col].get(*row)
                                )
                            })
                            .collect()
                    }

                    None => {
                        (0..ncols)
                            .map(|_| {
                                (
                                    "...".to_string(),
                                    Align::Center,
                                )
                            })
                            .collect()
                    }
                }
            })
            .collect();

        // --------------------------------------------
        // Column widths
        // --------------------------------------------

        let mut widths =
            Vec::with_capacity(ncols);

        for col in 0..ncols {
            let mut width = names[col].len();

            for row in &cells {
                width = width.max(row[col].0.len());
            }

            widths.push(width);
        }

        // --------------------------------------------
        // Header
        // --------------------------------------------

        for col in 0..ncols {
            if col > 0 {
                write!(f, " | ")?;
            }

            write!(
                f,
                "{:<width$}",
                names[col],
                width = widths[col]
            )?;
        }

        writeln!(f)?;

        // --------------------------------------------
        // Separator
        // --------------------------------------------

        for col in 0..ncols {
            if col > 0 {
                write!(f, "-+-")?;
            }

            write!(
                f,
                "{:-<width$}",
                "",
                width = widths[col]
            )?;
        }

        writeln!(f)?;

        // --------------------------------------------
        // Body
        // --------------------------------------------

        for row in &cells {
            for col in 0..ncols {
                if col > 0 {
                    write!(f, " | ")?;
                }

                let (text, align) = &row[col];

                match align {
                    Align::Left => {
                        write!(
                            f,
                            "{:<width$}",
                            text,
                            width = widths[col]
                        )?;
                    }

                    Align::Right => {
                        write!(
                            f,
                            "{:>width$}",
                            text,
                            width = widths[col]
                        )?;
                    }

                    Align::Center => {
                        write!(
                            f,
                            "{:^width$}",
                            text,
                            width = widths[col]
                        )?;
                    }
                }
            }

            writeln!(f)?;
        }

        Ok(())
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