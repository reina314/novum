use super::{DataFrame, DataFrameRef, Series, SeriesRef, Value};

use std::{collections::HashMap, fmt, rc::Rc};

pub type GroupedDataFrameRef = Rc<GroupedDataFrame>;

#[derive(Clone)]
pub struct GroupedDataFrame {
    dataframe: DataFrameRef,
    group_columns: Vec<String>,
    groups: Vec<Group>,
}

#[derive(Clone)]
pub struct Group {
    key: Vec<Value>,
    rows: Vec<usize>,
}

#[derive(Hash, Eq, PartialEq)]
enum CompositeGroupKey {
    Values(Vec<GroupKey>),
}

#[derive(Hash, Eq, PartialEq)]
enum GroupKey {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
    Null,
}

impl GroupedDataFrame {
    pub fn from_columns(dataframe: DataFrameRef, column_names: &[String]) -> Result<Self, String> {
        if column_names.is_empty() {
            return Err("group_by() requires at least one column".into());
        }

        let mut columns = Vec::with_capacity(column_names.len());

        for name in column_names {
            let column = dataframe
                .column(name)
                .ok_or_else(|| format!("unknown DataFrame column '{}'", name))?;

            columns.push(column);
        }

        let mut groups = Vec::<Group>::new();

        let mut positions = HashMap::<CompositeGroupKey, usize>::new();

        for row in 0..dataframe.nrows() {
            let mut key_values = Vec::with_capacity(columns.len());

            let mut key_parts = Vec::with_capacity(columns.len());

            for column in &columns {
                let value = column
                    .get(row)
                    .ok_or_else(|| format!("row index out of bounds: {}", row))?;

                key_parts.push(GroupKey::from_value(&value)?);

                key_values.push(value);
            }

            let key = CompositeGroupKey::Values(key_parts);

            if let Some(&group_index) = positions.get(&key) {
                groups[group_index].rows.push(row);
            } else {
                let group_index = groups.len();

                positions.insert(key, group_index);

                groups.push(Group {
                    key: key_values,
                    rows: vec![row],
                });
            }
        }

        Ok(Self {
            dataframe,
            group_columns: column_names.to_vec(),
            groups,
        })
    }

    pub fn from_column(dataframe: DataFrameRef, column_name: &str) -> Result<Self, String> {
        Self::from_columns(dataframe, &[column_name.to_owned()])
    }

    pub fn count(&self) -> Result<DataFrame, String> {
        let mut output_columns = Vec::<SeriesRef>::new();

        let key_columns = self.build_key_columns()?;

        output_columns.extend(key_columns);

        let counts = self
            .groups
            .iter()
            .map(|group| Value::Int(group.rows.len() as i64))
            .collect::<Vec<_>>();

        output_columns.push(Rc::new(Series::new("count", counts)));

        DataFrame::from_series(output_columns)
    }

    pub fn mean(&self, column_name: &str) -> Result<DataFrame, String> {
        let column = self
            .dataframe
            .column(column_name)
            .ok_or_else(|| format!("unknown DataFrame column '{}'", column_name))?;

        let values = self
            .groups
            .iter()
            .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Mean))
            .collect::<Result<Vec<_>, _>>()?;

        let mut output_columns = self.build_key_columns()?;

        output_columns.push(Rc::new(Series::new(
            format!("{}_mean", column_name),
            values,
        )));

        DataFrame::from_series(output_columns)
    }

    pub fn sum(&self, column_name: &str) -> Result<DataFrame, String> {
        let column = self
            .dataframe
            .column(column_name)
            .ok_or_else(|| format!("unknown DataFrame column '{}'", column_name))?;

        let values = self
            .groups
            .iter()
            .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Sum))
            .collect::<Result<Vec<_>, _>>()?;

        let mut output_columns = self.build_key_columns()?;

        output_columns.push(Rc::new(Series::new(format!("{}_sum", column_name), values)));

        DataFrame::from_series(output_columns)
    }

    pub fn aggregate(&self, column_name: &str, functions: &[String]) -> Result<DataFrame, String> {
        let column = self
            .dataframe
            .column(column_name)
            .ok_or_else(|| format!("unknown DataFrame column '{}'", column_name))?;

        if functions.is_empty() {
            return Err("aggregate() requires at least one function".into());
        }

        let mut output_columns = self.build_key_columns()?;

        for function in functions {
            let values = match function.as_str() {
                "count" => self
                    .groups
                    .iter()
                    .map(|group| {
                        let count = group
                            .rows
                            .iter()
                            .filter(|&&row| !matches!(column.get(row), Some(Value::Null)))
                            .count();

                        Ok(Value::Int(count as i64))
                    })
                    .collect::<Result<Vec<_>, String>>()?,

                "mean" => self
                    .groups
                    .iter()
                    .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Mean))
                    .collect::<Result<Vec<_>, _>>()?,

                "sum" => self
                    .groups
                    .iter()
                    .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Sum))
                    .collect::<Result<Vec<_>, _>>()?,

                "min" => self
                    .groups
                    .iter()
                    .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Min))
                    .collect::<Result<Vec<_>, _>>()?,

                "max" => self
                    .groups
                    .iter()
                    .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Max))
                    .collect::<Result<Vec<_>, _>>()?,

                "std" => self
                    .groups
                    .iter()
                    .map(|group| numeric_aggregate(&column, &group.rows, NumericAggregate::Std))
                    .collect::<Result<Vec<_>, _>>()?,

                other => {
                    return Err(format!("unknown aggregation '{}'", other));
                },
            };

            let output_name = format!("{}_{}", column_name, function);

            output_columns.push(Rc::new(Series::new(output_name, values)));
        }

        DataFrame::from_series(output_columns)
    }

    pub fn group_columns(&self) -> &[String] {
        &self.group_columns
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn build_key_columns(&self) -> Result<Vec<SeriesRef>, String> {
        let mut columns = Vec::with_capacity(self.group_columns.len());

        for (key_index, column_name) in self.group_columns.iter().enumerate() {
            let values = self
                .groups
                .iter()
                .map(|group| {
                    group
                        .key
                        .get(key_index)
                        .cloned()
                        .ok_or_else(|| format!("group key index out of bounds: {}", key_index))
                })
                .collect::<Result<Vec<_>, _>>()?;

            columns.push(Rc::new(Series::new(column_name.clone(), values)));
        }

        Ok(columns)
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
    let mut values = Vec::<f64>::with_capacity(rows.len());

    for &row in rows {
        let value = column
            .get(row)
            .ok_or_else(|| format!("row index out of bounds: {}", row))?;

        match value {
            Value::Int(value) => {
                values.push(value as f64);
            },

            Value::Float(value) => {
                values.push(value);
            },

            Value::Null => {},

            other => {
                return Err(format!(
                    "column '{}' is not numeric; found {}",
                    column.name(),
                    other.type_name()
                ));
            },
        }
    }

    if values.is_empty() {
        return Ok(Value::Null);
    }

    match aggregate {
        NumericAggregate::Mean => {
            let sum = values.iter().sum::<f64>();

            Ok(Value::Float(sum / values.len() as f64))
        },

        NumericAggregate::Sum => Ok(Value::Float(values.iter().sum::<f64>())),

        NumericAggregate::Min => {
            let value = values.into_iter().fold(f64::INFINITY, f64::min);

            Ok(Value::Float(value))
        },

        NumericAggregate::Max => {
            let value = values.into_iter().fold(f64::NEG_INFINITY, f64::max);

            Ok(Value::Float(value))
        },

        NumericAggregate::Std => {
            if values.len() < 2 {
                return Ok(Value::Null);
            }

            let mean = values.iter().sum::<f64>() / values.len() as f64;

            let variance = values
                .iter()
                .map(|value| {
                    let diff = *value - mean;

                    diff * diff
                })
                .sum::<f64>()
                / (values.len() - 1) as f64;

            Ok(Value::Float(variance.sqrt()))
        },
    }
}

impl GroupKey {
    fn from_value(value: &Value) -> Result<Self, String> {
        match value {
            Value::Int(value) => Ok(Self::Int(*value)),

            Value::Float(value) => Ok(Self::Float(value.to_bits())),

            Value::Bool(value) => Ok(Self::Bool(*value)),

            Value::Str(value) => Ok(Self::Str(value.as_ref().clone())),

            Value::Null => Ok(Self::Null),

            other => Err(format!("cannot group by {}", other.type_name())),
        }
    }
}

impl fmt::Debug for GroupedDataFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<grouped dataframe by {}: {} groups>",
            self.group_columns.join(", "),
            self.groups.len()
        )
    }
}
