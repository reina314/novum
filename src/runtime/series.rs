use super::{
    Value,
    Matrix,
    DataFrame,
};

use std::{
    fmt,
    rc::Rc,
};

pub type SeriesRef = Rc<Series>;

#[derive(Clone)]
pub struct Series {
    name: String,
    data: Vec<Value>,
}

impl Series {
    pub fn new(
        name: impl Into<String>,
        data: Vec<Value>,
    ) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Option<Value> {
        self.data.get(index).cloned()
    }

    pub fn data(&self) -> &[Value] {
        &self.data
    }

    pub fn into_values(self) -> Vec<Value> {
        self.data
    }

    fn numeric_values(&self) -> Vec<f64> {
        self.data
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

    fn ensure_numeric(
        &self,
    ) -> Result<Vec<f64>, String> {
        let mut values =
            Vec::with_capacity(
                self.data.len()
            );

        for value in &self.data {
            match value {
                Value::Int(v) =>
                    values.push(*v as f64),

                Value::Float(v) =>
                    values.push(*v),

                Value::Null => {
                    // Missing values are omitted.
                }

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                }
            }
        }

        Ok(values)
    }

    pub fn map_values<F>(
        &self,
        mut f: F,
    ) -> Self
    where
        F: FnMut(&Value) -> Value,
    {
        let data =
            self.data
                .iter()
                .map(&mut f)
                .collect();

        Self {
            name: self.name.clone(),
            data,
        }
    }

    pub fn with_name(
        &self,
        name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data: self.data.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_null(&self) -> Self {
        let data =
            self.data
                .iter()
                .map(|value| {
                    Value::Bool(
                        matches!(
                            value,
                            Value::Null
                        )
                    )
                })
                .collect();

        Self::new(
            self.name.clone(),
            data,
        )
    }

    pub fn is_not_null(&self) -> Self {
        let data =
            self.data
                .iter()
                .map(|value| {
                    Value::Bool(
                        !matches!(
                            value,
                            Value::Null
                        )
                    )
                })
                .collect();

        Self::new(
            self.name.clone(),
            data,
        )
    }

    pub fn to_matrix(
        &self,
    ) -> Result<Matrix, String> {
        let values =
            self.data()
                .iter()
                .map(|value| {
                    match value {
                        Value::Int(v) =>
                            Ok(*v as f64),

                        Value::Float(v) =>
                            Ok(*v),

                        Value::Null =>
                            Err(format!(
                                "Series '{}' contains Null",
                                self.name()
                            )),

                        other =>
                            Err(format!(
                                "Series '{}' is not numeric; found {}",
                                self.name(),
                                other.type_name()
                            )),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

        let rows =
            values
                .into_iter()
                .map(|value| vec![value])
                .collect();

        Matrix::from_rows(rows)
    }

    pub fn mean(
        &self,
    ) -> Result<Value, String> {
        let values =
            self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        let mean =
            values.iter().sum::<f64>()
            / values.len() as f64;

        Ok(Value::Float(mean))
    }

    pub fn sum(
        &self,
    ) -> Result<Value, String> {
        let values =
            self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        Ok(Value::Float(
            values.iter().sum()
        ))
    }

    pub fn min(
        &self,
    ) -> Result<Value, String> {
        let values =
            self.ensure_numeric()?;

        let result =
            values
                .into_iter()
                .reduce(f64::min);

        Ok(
            result
                .map(Value::Float)
                .unwrap_or(Value::Null)
        )
    }

    pub fn max(
        &self,
    ) -> Result<Value, String> {
        let values =
            self.ensure_numeric()?;

        let result =
            values
                .into_iter()
                .reduce(f64::max);

        Ok(
            result
                .map(Value::Float)
                .unwrap_or(Value::Null)
        )
    }

    pub fn std(
        &self,
    ) -> Result<Value, String> {
        let values =
            self.ensure_numeric()?;

        if values.len() < 2 {
            return Ok(Value::Null);
        }

        let mean =
            values.iter().sum::<f64>()
            / values.len() as f64;

        let sum_squared =
            values
                .iter()
                .map(|x| {
                    let diff =
                        *x - mean;

                    diff * diff
                })
                .sum::<f64>();

        let variance =
            sum_squared
            / (values.len() - 1) as f64;

        Ok(Value::Float(
            variance.sqrt()
        ))
    }

    pub fn median(
        &self,
    ) -> Result<Value, String> {
        let mut values =
            self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(
            |a, b| a.total_cmp(b)
        );

        let n =
            values.len();

        let result =
            if n % 2 == 1 {
                values[n / 2]
            } else {
                (
                    values[n / 2 - 1]
                    + values[n / 2]
                ) / 2.0
            };

        Ok(Value::Float(result))
    }

    pub fn quantile(
        &self,
        q: f64,
    ) -> Result<Value, String> {
        if !q.is_finite()
            || !(0.0..=1.0).contains(&q)
        {
            return Err(
                "quantile() expects q in [0, 1]"
                    .into()
            );
        }

        let mut values =
            self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(
            |a, b| a.total_cmp(b)
        );

        if values.len() == 1 {
            return Ok(
                Value::Float(values[0])
            );
        }

        let position =
            q * (values.len() - 1) as f64;

        let lower =
            position.floor() as usize;

        let upper =
            position.ceil() as usize;

        if lower == upper {
            return Ok(
                Value::Float(values[lower])
            );
        }

        let weight =
            position - lower as f64;

        let result =
            values[lower]
                * (1.0 - weight)
            + values[upper]
                * weight;

        Ok(Value::Float(result))
    }

    pub fn dropna(
        &self,
    ) -> Self {
        let data =
            self.data
                .iter()
                .filter(
                    |value| {
                        !matches!(
                            value,
                            Value::Null
                        )
                    }
                )
                .cloned()
                .collect();

        Self::new(
            self.name.clone(),
            data,
        )
    }

    pub fn unique(
        &self,
    ) -> Result<Self, String> {
        let mut result =
            Vec::<Value>::new();

        'outer:
        for value in &self.data {
            for existing in &result {
                if Value::eq_values(
                    value,
                    existing,
                )? {
                    continue 'outer;
                }
            }

            result.push(
                value.clone()
            );
        }

        Ok(
            Self::new(
                self.name.clone(),
                result,
            )
        )
    }

    pub fn value_counts(
        &self,
    ) -> Result<DataFrame, String> {
        let mut values =
            Vec::<Value>::new();

        let mut counts =
            Vec::<Value>::new();

        for value in &self.data {
            let mut found =
                None;

            for i in 0..values.len() {
                if Value::eq_values(
                    value,
                    &values[i],
                )? {
                    found = Some(i);
                    break;
                }
            }

            match found {
                Some(index) => {
                    if let Value::Int(count) =
                        &mut counts[index]
                    {
                        *count += 1;
                    }
                }

                None => {
                    values.push(
                        value.clone()
                    );

                    counts.push(
                        Value::Int(1)
                    );
                }
            }
        }

        DataFrame::from_series(
            vec![
                Rc::new(
                    Series::new(
                        "value",
                        values,
                    )
                ),
                Rc::new(
                    Series::new(
                        "count",
                        counts,
                    )
                ),
            ]
        )
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        const MAX_ITEMS: usize = 10;

        let data = self.data();

        write!(f, "{}: [", self.name())?;

        if data.len() <= MAX_ITEMS {
            for (i, value) in data.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }

                write!(f, "{value}")?;
            }
        } else {
            let head = MAX_ITEMS / 2;
            let tail = MAX_ITEMS - head;

            // First half
            for (i, value) in data.iter().take(head).enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }

                write!(f, "{value}")?;
            }

            write!(f, ", ...")?;

            // Last half
            for value in data.iter().skip(data.len() - tail) {
                write!(f, ", {value}")?;
            }
        }

        write!(f, "]")
    }
}

impl fmt::Debug for Series {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Series")
            .field("name", &self.name)
            .field("data", &self.data)
            .finish()
    }
}