use super::{
    DataFrameRef,
    GroupedDataFrameRef,
    List,
    ObjectRef,
    SeriesRef,
};

use std::fmt;

#[derive(Clone)]
pub enum MethodReceiver {
    List(List),
    Object(ObjectRef),
    Series(SeriesRef),
    DataFrame(DataFrameRef),
    GroupedDataFrame(GroupedDataFrameRef),
}

#[derive(Clone)]
pub struct BoundMethod {
    receiver: MethodReceiver,
    name: String,
}

impl BoundMethod {
    pub fn new(
        receiver: MethodReceiver,
        name: impl Into<String>,
    ) -> Self {
        Self {
            receiver,
            name: name.into(),
        }
    }

    pub fn receiver(&self)
        -> &MethodReceiver
    {
        &self.receiver
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Debug for BoundMethod {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<method {}>",
            self.name
        )
    }
}

impl fmt::Display for BoundMethod {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<method {}>",
            self.name
        )
    }
}