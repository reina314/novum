use super::{
    Series,
    SeriesRef,
    DataFrame,
    DataFrameRef,
    Value,
};

use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type GroupedDataFrameRef = Rc<GroupedDataFrame>;

#[derive(Clone)]
pub struct GroupedDataFrame {
    dataframe: DataFrameRef,
    group_column: String,
    groups: Vec<Group>,
}

#[derive(Clone)]
pub struct Group {
    key: Value,
    rows: Vec<usize>,
}

impl GroupedDataFrame {
    pub fn from_column(
        dataframe: DataFrameRef,
        column_name: &str,
    ) -> Result<Self, String> {
        let column =
            dataframe
                .column(column_name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_name
                    )
                })?;

        let mut groups =
            Vec::<Group>::new();

        let mut positions =
            HashMap::<GroupKey, usize>::new();

        for (row, value) in
            column.data().iter().enumerate()
        {
            let key =
                GroupKey::from_value(value)?;

            if let Some(&group_index) =
                positions.get(&key)
            {
                groups[group_index]
                    .rows
                    .push(row);
            } else {
                let group_index =
                    groups.len();

                positions.insert(
                    key,
                    group_index,
                );

                groups.push(
                    Group {
                        key: value.clone(),
                        rows: vec![row],
                    }
                );
            }
        }

        Ok(Self {
            dataframe,
            group_column:
                column_name.to_owned(),
            groups,
        })
    }

    pub fn count(
        &self,
    ) -> Result<DataFrame, String> {
        let mut keys =
            Vec::new();

        let mut counts =
            Vec::new();

        for group in &self.groups {
            keys.push(
                group.key.clone()
            );

            counts.push(
                Value::Int(
                    group.rows.len() as i64
                )
            );
        }

        let key_column =
            Rc::new(
                Series::new(
                    self.group_column.clone(),
                    keys,
                )
            );

        let count_column =
            Rc::new(
                Series::new(
                    "count",
                    counts,
                )
            );

        DataFrame::from_series(
            vec![
                key_column,
                count_column,
            ]
        )
    }

    pub fn mean(
        &self,
        column_name: &str,
    ) -> Result<DataFrame, String> {
        let column =
            self.dataframe
                .column(column_name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_name
                    )
                })?;

        let mut keys =
            Vec::new();

        let mut means =
            Vec::new();

        for group in &self.groups {
            let mut sum = 0.0;
            let mut count = 0usize;

            for &row in &group.rows {
                match column.get(row) {
                    Some(Value::Int(v)) => {
                        sum += v as f64;
                        count += 1;
                    }

                    Some(Value::Float(v)) => {
                        sum += v;
                        count += 1;
                    }

                    Some(Value::Null) => {
                        // omit missing
                    }

                    Some(other) => {
                        return Err(format!(
                            "column '{}' is not numeric; found {}",
                            column_name,
                            other.type_name()
                        ));
                    }

                    None => {
                        return Err(format!(
                            "row index out of bounds: {}",
                            row
                        ));
                    }
                }
            }

            keys.push(
                group.key.clone()
            );

            if count == 0 {
                means.push(
                    Value::Null
                );
            } else {
                means.push(
                    Value::Float(
                        sum / count as f64
                    )
                );
            }
        }

        DataFrame::from_series(
            vec![
                Rc::new(
                    Series::new(
                        self.group_column.clone(),
                        keys,
                    )
                ),
                Rc::new(
                    Series::new(
                        format!(
                            "{}_mean",
                            column_name
                        ),
                        means,
                    )
                ),
            ]
        )
    }

    pub fn sum(
        &self,
        column_name: &str,
    ) -> Result<DataFrame, String> {
        let column =
            self.dataframe
                .column(column_name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_name
                    )
                })?;

        let mut keys =
            Vec::new();

        let mut sums =
            Vec::new();

        for group in &self.groups {
            let mut sum = 0.0;
            let mut count = 0usize;

            for &row in &group.rows {
                match column.get(row) {
                    Some(Value::Int(v)) => {
                        sum += v as f64;
                        count += 1;
                    }

                    Some(Value::Float(v)) => {
                        sum += v;
                        count += 1;
                    }

                    Some(Value::Null) => {}

                    Some(other) => {
                        return Err(format!(
                            "column '{}' is not numeric; found {}",
                            column_name,
                            other.type_name()
                        ));
                    }

                    None => {
                        return Err(format!(
                            "row index out of bounds: {}",
                            row
                        ));
                    }
                }
            }

            keys.push(
                group.key.clone()
            );

            if count == 0 {
                sums.push(
                    Value::Null
                );
            } else {
                sums.push(
                    Value::Float(sum)
                );
            }
        }

        DataFrame::from_series(
            vec![
                Rc::new(
                    Series::new(
                        self.group_column.clone(),
                        keys,
                    )
                ),
                Rc::new(
                    Series::new(
                        format!(
                            "{}_sum",
                            column_name
                        ),
                        sums,
                    )
                ),
            ]
        )
    }

    pub fn aggregate(
        &self,
        column_name: &str,
        functions: &[String],
    ) -> Result<DataFrame, String> {
        let column =
            self.dataframe
                .column(column_name)
                .ok_or_else(|| {
                    format!(
                        "unknown DataFrame column '{}'",
                        column_name
                    )
                })?;

        let mut output_columns =
            Vec::<SeriesRef>::new();

        // Group key column
        let keys =
            self.groups
                .iter()
                .map(|group| group.key.clone())
                .collect::<Vec<_>>();

        output_columns.push(
            Rc::new(
                Series::new(
                    self.group_column.clone(),
                    keys,
                )
            )
        );

        for function in functions {
            match function.as_str() {
                "count" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                let count =
                                    group.rows
                                        .iter()
                                        .filter(|&&row| {
                                            !matches!(
                                                column.get(row),
                                                Some(Value::Null)
                                            )
                                        })
                                        .count();

                                Value::Int(
                                    count as i64
                                )
                            })
                            .collect();

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_count",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                "mean" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                numeric_aggregate(
                                    &column,
                                    &group.rows,
                                    NumericAggregate::Mean,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_mean",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                "sum" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                numeric_aggregate(
                                    &column,
                                    &group.rows,
                                    NumericAggregate::Sum,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_sum",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                "min" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                numeric_aggregate(
                                    &column,
                                    &group.rows,
                                    NumericAggregate::Min,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_min",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                "max" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                numeric_aggregate(
                                    &column,
                                    &group.rows,
                                    NumericAggregate::Max,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_max",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                "std" => {
                    let values =
                        self.groups
                            .iter()
                            .map(|group| {
                                numeric_aggregate(
                                    &column,
                                    &group.rows,
                                    NumericAggregate::Std,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                    output_columns.push(
                        Rc::new(
                            Series::new(
                                format!(
                                    "{}_std",
                                    column_name
                                ),
                                values,
                            )
                        )
                    );
                }

                other => {
                    return Err(format!(
                        "unknown aggregation '{}'",
                        other
                    ));
                }
            }
        }

        DataFrame::from_series(
            output_columns
        )
    }

    pub fn group_column(
        &self,
    ) -> &str {
        &self.group_column
    }

    pub fn group_count(
        &self,
    ) -> usize {
        self.groups.len()
    }
}

enum NumericAggregate {
    Mean,
    Sum,
    Min,
    Max,
    Std,
}

fn numeric_aggregate(
    column: &SeriesRef,
    rows: &[usize],
    aggregate: NumericAggregate,
) -> Result<Value, String> {
    let values =
        rows.iter()
            .filter_map(|&row| {
                match column.get(row) {
                    Some(Value::Int(v)) =>
                        Some(v as f64),

                    Some(Value::Float(v)) =>
                        Some(v),

                    Some(Value::Null) =>
                        None,

                    Some(_) =>
                        None,

                    None =>
                        None,
                }
            })
            .collect::<Vec<_>>();

    if values.is_empty() {
        return Ok(Value::Null);
    }

    match aggregate {
        NumericAggregate::Mean => {
            Ok(Value::Float(
                values.iter().sum::<f64>()
                    / values.len() as f64
            ))
        }

        NumericAggregate::Sum => {
            Ok(Value::Float(
                values.iter().sum()
            ))
        }

        NumericAggregate::Min => {
            Ok(Value::Float(
                values
                    .into_iter()
                    .fold(
                        f64::INFINITY,
                        f64::min
                    )
            ))
        }

        NumericAggregate::Max => {
            Ok(Value::Float(
                values
                    .into_iter()
                    .fold(
                        f64::NEG_INFINITY,
                        f64::max
                    )
            ))
        }

        NumericAggregate::Std => {
            if values.len() < 2 {
                return Ok(Value::Null);
            }

            let mean =
                values.iter().sum::<f64>()
                    / values.len() as f64;

            let variance =
                values
                    .iter()
                    .map(|x| {
                        (x - mean).powi(2)
                    })
                    .sum::<f64>()
                    / (values.len() - 1) as f64;

            Ok(
                Value::Float(
                    variance.sqrt()
                )
            )
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
enum GroupKey {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
    Null,
}

impl GroupKey {
    fn from_value(
        value: &Value,
    ) -> Result<Self, String> {
        match value {
            Value::Int(v) =>
                Ok(Self::Int(*v)),

            Value::Float(v) =>
                Ok(Self::Float(
                    v.to_bits()
                )),

            Value::Bool(v) =>
                Ok(Self::Bool(*v)),

            Value::Str(v) =>
                Ok(Self::Str(
                    v.as_ref().clone()
                )),

            Value::Null =>
                Ok(Self::Null),

            other => {
                Err(format!(
                    "cannot group by {}",
                    other.type_name()
                ))
            }
        }
    }
}

impl fmt::Debug for GroupedDataFrame {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<grouped dataframe by '{}': {} groups>",
            self.group_column,
            self.groups.len()
        )
    }
}