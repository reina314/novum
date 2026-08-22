use super::{
    DataFrameRef,
    GroupedDataFrameRef,
    List,
    VectorRef,
    MatrixRef,
    ObjectRef,
    SeriesRef,
    IteratorRef,
};

use std::{
    fmt,
    rc::Rc,
};

#[derive(Clone)]
pub enum MethodReceiver {
    Str(Rc<String>),
    List(List),
    Vector(VectorRef),
    Matrix(MatrixRef),

    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },

    Object(ObjectRef),
    Series(SeriesRef),
    DataFrame(DataFrameRef),
    GroupedDataFrame(GroupedDataFrameRef),
    Iterator(IteratorRef),
}

impl MethodReceiver {
    pub fn supports_method(
        &self,
        name: &str,
    ) -> bool {
        match self {
            Self::List(_) => matches!(
                name,
                "push"
                    | "pop"
                    | "remove"
                    | "len"
                    | "iter"
                    | "get"
                    | "set"
                    | "insert"
                    | "contains"
                    | "reverse"
                    | "clear"
                    | "extend"
                    | "join"
                    | "vector"
            ),

            Self::Str(_) => matches!(
                name,
                "chars"
                    | "len"
                    | "trim"
                    | "to_upper"
                    | "to_lower"
                    | "contains"
                    | "starts_with"
                    | "ends_with"
                    | "split"
                    | "replace"
                    | "repeat"
            ),

            Self::Vector(_) => matches!(
                name,
                "len"
                    | "shape"
                    | "norm"
                    | "dot"
                    | "to_matrix"
            ),

            Self::Matrix(_) => matches!(
                name,
                "shape"
                    | "transpose"
                    | "trace"
            ),

            Self::Iterator(_) => matches!(
                name,
                "next"
                    | "map"
                    | "filter"
                    | "collect"
                    | "reduce"
                    | "fold"
                    | "any"
                    | "all"
                    | "enumerate"
                    | "zip"
                    | "take"
                    | "skip"
            ),

            Self::Range { .. } => matches!(
                name,
                "iter"
                    | "map"
                    | "filter"
                    | "collect"
                    | "reduce"
                    | "fold"
                    | "any"
                    | "all"
                    | "enumerate"
                    | "zip"
                    | "take"
                    | "skip"
            ),

            // Existing cases...
            _ => false,
        }
    }
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