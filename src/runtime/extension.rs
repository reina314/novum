use super::Value;

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReceiverKind {
    Any,

    Int,
    Float,
    Bool,
    Str,

    Tuple,
    List,
    Set,
    Dict,

    Vector,
    Matrix,

    Series,
    DataFrame,

    Object,
}

#[derive(Clone)]
pub struct ExtensionMethod {
    pub value: Value,
}

pub struct ExtensionRegistry {
    methods: HashMap<(ReceiverKind, String), ExtensionMethod>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        receiver: ReceiverKind,
        name: impl Into<String>,
        value: Value,
    ) {
        self.methods.insert(
            (receiver, name.into()),
            ExtensionMethod { value },
        );
    }

    pub fn get(
        &self,
        receiver: ReceiverKind,
        name: &str,
    ) -> Option<&Value> {
        self.methods
            .get(&(receiver, name.to_string()))
            .map(|method| &method.value)
            .or_else(|| {
                self.methods
                    .get(&(
                        ReceiverKind::Any,
                        name.to_string(),
                    ))
                    .map(|method| &method.value)
            })
    }
}