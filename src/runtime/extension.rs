use super::{ClosureRef, IterResult, IteratorRef, Value};

use crate::{error::Result, syntax::BinOp};

use std::collections::HashMap;

pub trait ExtensionHost {
    fn make_iterator(&self, value: Value) -> Result<IteratorRef>;

    fn iterator_next(&mut self, iterator: IteratorRef) -> Result<IterResult>;

    fn collect_iterator(&mut self, iterator: IteratorRef) -> Result<Value>;

    fn reduce_iterator(&mut self, iterator: IteratorRef, closure: ClosureRef) -> Result<Value>;

    fn fold_iterator(
        &mut self,
        iterator: IteratorRef,
        initial: Value,
        closure: ClosureRef,
    ) -> Result<Value>;

    fn any_iterator(&mut self, iterator: IteratorRef, closure: ClosureRef) -> Result<Value>;

    fn all_iterator(&mut self, iterator: IteratorRef, closure: ClosureRef) -> Result<Value>;

    fn numeric_reduce(&mut self, iterator: IteratorRef, op: BinOp) -> Result<Value>;

    fn extreme_iterator(&mut self, iterator: IteratorRef, maximum: bool) -> Result<Value>;

    fn call_closure_sync_named(
        &mut self,
        closure: ClosureRef,
        args: Vec<Value>,
        names: &[Option<String>],
    ) -> Result<Value>;
}

pub type NativeExtensionFn =
    fn(&mut dyn ExtensionHost, Value, Vec<Value>, &[Option<String>]) -> Result<Value>;

#[derive(Clone)]
pub enum ExtensionTarget {
    Callable(Value),
    Native(NativeExtensionFn),
}

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
    GroupedDataFrame,

    Iterator,
    Range,
    Path,

    Object,
}

pub struct ExtensionRegistry {
    methods: HashMap<(ReceiverKind, String), ExtensionTarget>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    pub fn register(&mut self, receiver: ReceiverKind, name: impl Into<String>, value: Value) {
        self.methods
            .insert((receiver, name.into()), ExtensionTarget::Callable(value));
    }

    pub fn register_native(
        &mut self,
        receiver: ReceiverKind,
        name: impl Into<String>,
        function: NativeExtensionFn,
    ) {
        self.methods
            .insert((receiver, name.into()), ExtensionTarget::Native(function));
    }

    pub fn get(&self, receiver: ReceiverKind, name: &str) -> Option<&ExtensionTarget> {
        self.methods
            .get(&(receiver, name.to_string()))
            .or_else(|| self.methods.get(&(ReceiverKind::Any, name.to_string())))
    }

    pub fn register_numeric(&mut self, name: impl Into<String>, value: Value) {
        let name = name.into();

        self.register(ReceiverKind::Int, name.clone(), value.clone());

        self.register(ReceiverKind::Float, name, value);
    }
}
