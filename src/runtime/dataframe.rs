use super::{
    Matrix,
    Series,
    SeriesRef,
    Object,
    ObjectRef,
    Value,
};

use std::{
    fmt,
    rc::Rc,
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
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

    pub fn numeric_columns(
        &self,
    ) -> Vec<SeriesRef> {
        self.columns
            .iter()
            .filter(|column| {
                column.data()
                    .iter()
                    .all(|value| {
                        matches!(
                            value,
                            Value::Int(_)
                            | Value::Float(_)
                            | Value::Null
                        )
                    })
            })
            .cloned()
            .collect()
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

    pub fn sort_by_column(
        &self,
        column_name: &str,
        ascending: bool,
    ) -> Result<Self, String> {
        let column =
            self.column(column_name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_name
                    )
                })?;

        let mut indices =
            (0..self.nrows)
                .collect::<Vec<_>>();

        indices.sort_by(|&a, &b| {
            let va =
                column
                    .get(a)
                    .unwrap_or(Value::Null);

            let vb =
                column
                    .get(b)
                    .unwrap_or(Value::Null);

            let ordering =
                compare_values_for_sort(
                    &va,
                    &vb,
                );

            if ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

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

    pub fn rename(
        &self,
        mapping: &std::collections::HashMap<String, String>,
    ) -> Result<Self, String> {
        let mut columns =
            Vec::with_capacity(
                self.columns.len()
            );

        for column in &self.columns {
            let new_name =
                mapping
                    .get(column.name())
                    .cloned()
                    .unwrap_or_else(|| {
                        column.name().to_owned()
                    });

            columns.push(
                Rc::new(
                    Series::new(
                        new_name,
                        column.data().to_vec(),
                    )
                )
            );
        }

        Self::from_series(columns)
    }

    pub fn drop_columns(
        &self,
        names: &[String],
    ) -> Result<Self, String> {
        if names.is_empty() {
            return Ok(self.clone());
        }

        let columns = self
            .columns
            .iter()
            .filter(|column| {
                !names.iter().any(|name| {
                    name == column.name()
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        if columns.is_empty() {
            return Err(
                "drop() cannot remove all DataFrame columns"
                    .into()
            );
        }

        Self::from_series(columns)
    }

    pub fn describe(
        &self,
    ) -> Result<Self, String> {
        let numeric_columns = self
            .columns
            .iter()
            .filter(|column| {
                is_numeric_column(column)
            })
            .collect::<Vec<_>>();

        if numeric_columns.is_empty() {
            return Err(
                "describe() found no numeric columns"
                    .into()
            );
        }

        let mut column_names =
            Vec::<Value>::new();

        let mut counts =
            Vec::<Value>::new();

        let mut means =
            Vec::<Value>::new();

        let mut stds =
            Vec::<Value>::new();

        let mut mins =
            Vec::<Value>::new();

        let mut medians =
            Vec::<Value>::new();

        let mut maxs =
            Vec::<Value>::new();

        for column in numeric_columns {
            let values =
                numeric_values(column);

            column_names.push(
                Value::Str(
                    Rc::new(
                        column.name().to_owned()
                    )
                )
            );

            counts.push(
                Value::Int(
                    values.len() as i64
                )
            );

            if values.is_empty() {
                means.push(Value::Null);
                stds.push(Value::Null);
                mins.push(Value::Null);
                medians.push(Value::Null);
                maxs.push(Value::Null);

                continue;
            }

            // -----------------------------------------------------
            // Mean
            // -----------------------------------------------------

            let mean =
                values.iter().sum::<f64>()
                / values.len() as f64;

            means.push(
                Value::Float(mean)
            );

            // -----------------------------------------------------
            // Standard deviation
            // -----------------------------------------------------

            if values.len() >= 2 {
                let sum_squared =
                    values
                        .iter()
                        .map(|value| {
                            let diff =
                                *value - mean;

                            diff * diff
                        })
                        .sum::<f64>();

                let variance =
                    sum_squared
                        / (values.len() - 1) as f64;

                stds.push(
                    Value::Float(
                        variance.sqrt()
                    )
                );
            } else {
                stds.push(Value::Null);
            }

            // -----------------------------------------------------
            // Sorted values
            // -----------------------------------------------------

            let mut sorted =
                values.clone();

            sorted.sort_by(
                |a, b| a.total_cmp(b)
            );

            // -----------------------------------------------------
            // Min / Max
            // -----------------------------------------------------

            mins.push(
                Value::Float(
                    sorted[0]
                )
            );

            maxs.push(
                Value::Float(
                    sorted[sorted.len() - 1]
                )
            );

            // -----------------------------------------------------
            // Median
            // -----------------------------------------------------

            let n =
                sorted.len();

            let median =
                if n % 2 == 1 {
                    sorted[n / 2]
                } else {
                    let upper =
                        n / 2;

                    (
                        sorted[upper - 1]
                        + sorted[upper]
                    ) / 2.0
                };

            medians.push(
                Value::Float(median)
            );
        }

        Self::from_series(vec![
            Rc::new(
                Series::new(
                    "column",
                    column_names,
                )
            ),

            Rc::new(
                Series::new(
                    "count",
                    counts,
                )
            ),

            Rc::new(
                Series::new(
                    "mean",
                    means,
                )
            ),

            Rc::new(
                Series::new(
                    "std",
                    stds,
                )
            ),

            Rc::new(
                Series::new(
                    "min",
                    mins,
                )
            ),

            Rc::new(
                Series::new(
                    "median",
                    medians,
                )
            ),

            Rc::new(
                Series::new(
                    "max",
                    maxs,
                )
            ),
        ])
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

        let mut columns =
            Vec::with_capacity(
                self.columns.len()
            );

        for column in
            &self.columns
        {
            let values =
                column
                    .data()
                    .iter()
                    .take(end)
                    .cloned()
                    .collect();

            columns.push(
                Rc::new(
                    Series::new(
                        column.name(),
                        values,
                    )
                )
            );
        }

        Self::from_series(
            columns
        )
    }

    pub fn crosstab(
        &self,
        row_column: &str,
        column_column: &str,
    ) -> Result<Self, String> {
        let row_series =
            self.column(row_column)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        row_column
                    )
                })?;

        let col_series =
            self.column(column_column)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_column
                    )
                })?;

        if row_series.len() != col_series.len() {
            return Err(
                "crosstab columns have different lengths"
                    .into()
            );
        }

        // ---------------------------------------------------------
        // Discover unique row/column categories.
        //
        // Preserve first-seen order rather than sorting them.
        // ---------------------------------------------------------

        let mut row_values =
            Vec::<Value>::new();

        let mut column_values =
            Vec::<Value>::new();

        for i in 0..self.nrows {
            let row_value =
                row_series.get(i).unwrap();

            let column_value =
                col_series.get(i).unwrap();

            if !contains_value(
                &row_values,
                &row_value,
            )? {
                row_values.push(
                    row_value.clone()
                );
            }

            if !contains_value(
                &column_values,
                &column_value,
            )? {
                column_values.push(
                    column_value.clone()
                );
            }
        }

        // ---------------------------------------------------------
        // Count matrix
        // ---------------------------------------------------------

        let mut counts =
            vec![
                vec![0i64; column_values.len()];
                row_values.len()
            ];

        for i in 0..self.nrows {
            let row_value =
                row_series.get(i).unwrap();

            let column_value =
                col_series.get(i).unwrap();

            let row_index =
                find_value(
                    &row_values,
                    &row_value,
                )?;

            let column_index =
                find_value(
                    &column_values,
                    &column_value,
                )?;

            counts[row_index][column_index] += 1;
        }

        // ---------------------------------------------------------
        // Build DataFrame
        //
        // First column = row category.
        // Other columns = column categories.
        // ---------------------------------------------------------

        let mut output =
            Vec::<SeriesRef>::new();

        output.push(
            Rc::new(
                Series::new(
                    row_column,
                    row_values.clone(),
                )
            )
        );

        for (column_index, column_value)
            in column_values.iter().enumerate()
        {
            let name =
                value_to_column_name(
                    column_value
                )?;

            let data =
                counts
                    .iter()
                    .map(|row| {
                        Value::Int(
                            row[column_index]
                        )
                    })
                    .collect::<Vec<_>>();

            output.push(
                Rc::new(
                    Series::new(
                        name,
                        data,
                    )
                )
            );
        }

        Self::from_series(output)
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

/// Helper for `describe()`
fn numeric_values(
    column: &Series,
) -> Vec<f64> {
    column
        .data()
        .iter()
        .filter_map(|value| {
            match value {
                Value::Int(v) =>
                    Some(*v as f64),

                Value::Float(v) =>
                    Some(*v),

                Value::Null =>
                    None,

                _ =>
                    None,
            }
        })
        .collect()
}

/// Helper for `describe()`
fn is_numeric_column(
    column: &Series,
) -> bool {
    column
        .data()
        .iter()
        .all(|value| {
            matches!(
                value,
                Value::Int(_)
                    | Value::Float(_)
                    | Value::Null
            )
        })
}

/// Helper for `crosstab()`
fn contains_value(
    values: &[Value],
    target: &Value,
) -> Result<bool, String> {
    for value in values {
        if Value::eq_values(
            value,
            target,
        )? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Helper for `crosstab()`
fn find_value(
    values: &[Value],
    target: &Value,
) -> Result<usize, String> {
    for (index, value) in
        values.iter().enumerate()
    {
        if Value::eq_values(
            value,
            target,
        )? {
            return Ok(index);
        }
    }

    Err(
        "internal crosstab category lookup failure"
            .into()
    )
}

/// Helper for `crosstab()`
fn value_to_column_name(
    value: &Value,
) -> Result<String, String> {
    match value {
        Value::Str(value) =>
            Ok(value.as_ref().clone()),

        Value::Int(value) =>
            Ok(value.to_string()),

        Value::Float(value) =>
            Ok(value.to_string()),

        Value::Bool(value) =>
            Ok(value.to_string()),

        Value::Null =>
            Ok("null".into()),

        other => {
            Err(format!(
                "crosstab category cannot be used as column name: {}",
                other.type_name()
            ))
        }
    }
}

fn compare_values_for_sort(
    a: &Value,
    b: &Value,
) -> Ordering {
    match (a, b) {
        (Value::Null, Value::Null) =>
            Ordering::Equal,

        // Put Null at the end.
        (Value::Null, _) =>
            Ordering::Greater,

        (_, Value::Null) =>
            Ordering::Less,

        (Value::Int(a), Value::Int(b)) =>
            a.cmp(b),

        (Value::Float(a), Value::Float(b)) =>
            a.total_cmp(b),

        (Value::Int(a), Value::Float(b)) =>
            (*a as f64).total_cmp(b),

        (Value::Float(a), Value::Int(b)) =>
            a.total_cmp(&(*b as f64)),

        (Value::Str(a), Value::Str(b)) =>
            a.cmp(b),

        (Value::Bool(a), Value::Bool(b)) =>
            a.cmp(b),

        // Different types:
        // deterministic ordering by type name.
        (a, b) =>
            a.type_name()
                .cmp(b.type_name()),
    }
}

