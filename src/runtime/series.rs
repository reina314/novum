use super::{
    Value,
    Matrix
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

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
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