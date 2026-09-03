use super::{
    Matrix,
    // DataFrame,
    Value,
};

use std::{fmt, rc::Rc};

pub type SeriesRef = Rc<Series>;

#[derive(Clone)]
pub struct Series {
    name: String,
    data: Vec<Value>,
}

impl Series {
    pub fn new(name: impl Into<String>, data: Vec<Value>) -> Self {
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

    pub fn get(&self, index: usize) -> Option<Value> {
        self.data.get(index).cloned()
    }

    pub fn data(&self) -> &[Value] {
        &self.data
    }

    pub fn into_values(self) -> Vec<Value> {
        self.data
    }

    pub fn slice(&self, range: std::ops::Range<usize>) -> Result<Self, String> {
        if range.start > range.end {
            return Err("slice start must not exceed end".into());
        }

        if range.end > self.data.len() {
            return Err(format!(
                "slice end {} out of bounds for length {}",
                range.end,
                self.data.len()
            ));
        }

        Ok(Self::new(self.name.clone(), self.data[range].to_vec()))
    }

    fn ensure_numeric(&self) -> Result<Vec<f64>, String> {
        self.numeric_values()
    }

    pub fn numeric_values(&self) -> Result<Vec<f64>, String> {
        let mut values = Vec::with_capacity(self.data.len());

        for value in &self.data {
            match value {
                Value::Int(v) => {
                    values.push(*v as f64);
                },

                Value::Float(v) => {
                    values.push(*v);
                },

                Value::Null => {
                    // Missing values are omitted.
                },

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            }
        }

        Ok(values)
    }

    pub fn map_values<F>(&self, mut f: F) -> Self
    where
        F: FnMut(&Value) -> Value,
    {
        let data = self.data.iter().map(&mut f).collect();

        Self {
            name: self.name.clone(),
            data,
        }
    }

    pub fn with_name(&self, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: self.data.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_null(&self) -> Self {
        let data = self
            .data
            .iter()
            .map(|value| Value::Bool(matches!(value, Value::Null)))
            .collect();

        Self::new(self.name.clone(), data)
    }

    pub fn is_not_null(&self) -> Self {
        let data = self
            .data
            .iter()
            .map(|value| Value::Bool(!matches!(value, Value::Null)))
            .collect();

        Self::new(self.name.clone(), data)
    }

    pub fn to_matrix(&self) -> Result<Matrix, String> {
        let values = self
            .data()
            .iter()
            .map(|value| match value {
                Value::Int(v) => Ok(*v as f64),

                Value::Float(v) => Ok(*v),

                Value::Null => Err(format!("Series '{}' contains Null", self.name())),

                other => Err(format!(
                    "Series '{}' is not numeric; found {}",
                    self.name(),
                    other.type_name()
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let rows = values.into_iter().map(|value| vec![value]).collect();

        Matrix::from_rows(rows)
    }

    pub fn mean(&self) -> Result<Value, String> {
        let mut sum = 0.0;
        let mut count = 0usize;

        for value in &self.data {
            match value {
                Value::Int(v) => {
                    sum += *v as f64;
                    count += 1;
                },

                Value::Float(v) => {
                    sum += *v;
                    count += 1;
                },

                Value::Null => {
                    // Ignore missing values.
                },

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            }
        }

        if count == 0 {
            return Ok(Value::Null);
        }

        Ok(Value::Float(sum / count as f64))
    }

    pub fn sum(&self) -> Result<Value, String> {
        let mut sum = 0.0;
        let mut count = 0usize;

        for value in &self.data {
            match value {
                Value::Int(v) => {
                    sum += *v as f64;
                    count += 1;
                },

                Value::Float(v) => {
                    sum += *v;
                    count += 1;
                },

                Value::Null => {},

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            }
        }

        if count == 0 {
            return Ok(Value::Null);
        }

        Ok(Value::Float(sum))
    }

    pub fn min(&self) -> Result<Value, String> {
        let mut result: Option<f64> = None;

        for value in &self.data {
            let current = match value {
                Value::Int(v) => *v as f64,

                Value::Float(v) => *v,

                Value::Null => continue,

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            };

            result = Some(match result {
                Some(current_min) => current_min.min(current),

                None => current,
            });
        }

        Ok(result.map(Value::Float).unwrap_or(Value::Null))
    }

    pub fn max(&self) -> Result<Value, String> {
        let mut result: Option<f64> = None;

        for value in &self.data {
            let current = match value {
                Value::Int(v) => *v as f64,

                Value::Float(v) => *v,

                Value::Null => continue,

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            };

            result = Some(match result {
                Some(current_max) => current_max.max(current),

                None => current,
            });
        }

        Ok(result.map(Value::Float).unwrap_or(Value::Null))
    }

    pub fn std(&self) -> Result<Value, String> {
        let mut sum = 0.0;
        let mut count = 0usize;

        for value in &self.data {
            match value {
                Value::Int(v) => {
                    sum += *v as f64;
                    count += 1;
                },

                Value::Float(v) => {
                    sum += *v;
                    count += 1;
                },

                Value::Null => {},

                other => {
                    return Err(format!(
                        "Series '{}' is not numeric; found {}",
                        self.name,
                        other.type_name()
                    ));
                },
            }
        }

        if count < 2 {
            return Ok(Value::Null);
        }

        let mean = sum / count as f64;

        let mut sum_squared = 0.0;

        for value in &self.data {
            let x = match value {
                Value::Int(v) => *v as f64,

                Value::Float(v) => *v,

                Value::Null => continue,

                _ => unreachable!("numeric validation must have succeeded"),
            };

            let diff = x - mean;

            sum_squared += diff * diff;
        }

        let variance = sum_squared / (count - 1) as f64;

        Ok(Value::Float(variance.sqrt()))
    }

    pub fn median(&self) -> Result<Value, String> {
        let mut values = self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(|a, b| a.total_cmp(b));

        let n = values.len();

        let result = if n % 2 == 1 {
            values[n / 2]
        } else {
            (values[n / 2 - 1] + values[n / 2]) / 2.0
        };

        Ok(Value::Float(result))
    }

    pub fn quantile(&self, q: f64) -> Result<Value, String> {
        if !q.is_finite() || !(0.0..=1.0).contains(&q) {
            return Err("quantile() expects q in [0, 1]".into());
        }

        let mut values = self.ensure_numeric()?;

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(|a, b| a.total_cmp(b));

        if values.len() == 1 {
            return Ok(Value::Float(values[0]));
        }

        let position = q * (values.len() - 1) as f64;

        let lower = position.floor() as usize;

        let upper = position.ceil() as usize;

        if lower == upper {
            return Ok(Value::Float(values[lower]));
        }

        let weight = position - lower as f64;

        let result = values[lower] * (1.0 - weight) + values[upper] * weight;

        Ok(Value::Float(result))
    }

    pub fn dropna(&self) -> Self {
        let data = self
            .data
            .iter()
            .filter(|value| !matches!(value, Value::Null))
            .cloned()
            .collect();

        Self::new(self.name.clone(), data)
    }

    pub fn unique(&self) -> Result<Self, String> {
        let mut result = Vec::<Value>::new();

        'outer: for value in &self.data {
            for existing in &result {
                if Value::eq_values(value, existing)? {
                    continue 'outer;
                }
            }

            result.push(value.clone());
        }

        Ok(Self::new(self.name.clone(), result))
    }

    pub fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Series")
            .field("name", &self.name)
            .field("data", &self.data)
            .finish()
    }
}
