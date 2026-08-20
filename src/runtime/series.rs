use super::Value;

use std::{
    fmt,
    rc::Rc,
};

pub type SeriesRef =
    Rc<Series>;

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

    pub fn data(&self)
        -> &[Value]
    {
        &self.data
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