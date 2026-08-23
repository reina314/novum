use super::{
    DataFrameRef,
    GroupedDataFrameRef,
    Value,
    List,
    Dict,
    SetRef,
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
    Set(SetRef),
    Dict(Dict),
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

            Self::Set(_) => matches!(
                name,
                "len"
                    | "add"
                    | "remove"
                    | "contains"
                    | "clear"
                    | "iter"
                    | "union"
                    | "intersection"
                    | "difference"
            ),

            Self::Dict(_) => matches!(
                name,
                "get"
                    | "set"
                    | "remove"
                    | "contains"
                    | "keys"
                    | "values"
                    | "len"
                    | "items"
                    | "iter"
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

    pub fn is_iterator_method(
        &self,
        name: &str,
    ) -> bool {
        matches!(
            name,
            "map"
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
        )
    }

    pub fn to_iterable_value(
        self,
    ) -> Option<Value> {
        match self {
            Self::List(list) =>
                Some(
                    Value::List(list)
                ),

            Self::Dict(dict) =>
                Some(
                    Value::Dict(dict)
                ),

            Self::Str(string) =>
                Some(
                    Value::Str(string)
                ),

            Self::Vector(vector) =>
                Some(
                    Value::Vector(vector)
                ),

            Self::Set(set) =>
                Some(
                    Value::Set(set)
                ),

            Self::Range {
                start,
                end,
                inclusive,
            } =>
                Some(
                    Value::Range(
                        start,
                        end,
                        inclusive,
                    )
                ),

            Self::Iterator(iterator) =>
                Some(
                    Value::Iterator(iterator)
                ),

            _ =>
                None,
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