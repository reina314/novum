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