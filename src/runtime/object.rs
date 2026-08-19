use super::{FuncRef, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type ObjectRef = Rc<RefCell<Object>>;

#[derive(Clone)]
pub struct ObjectMethod {
    pub object: ObjectRef,
    pub name: String,
    pub function: FuncRef,
}

#[derive(Clone)]
pub struct Object {
    type_name: String,
    fields: HashMap<String, Value>,
    methods: HashMap<String, FuncRef>,
}

impl Object {
    pub fn new() -> Self {
        Self {
            type_name: "Object".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn set_type_name(
        &mut self,
        name: impl Into<String>,
    ) {
        self.type_name = name.into();
    }

    pub fn get_field(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.fields.get(name).cloned()
    }

    pub fn set_field(
        &mut self,
        name: impl Into<String>,
        value: Value,
    ) {
        self.fields.insert(
            name.into(),
            value,
        );
    }

    pub fn has_field(
        &self,
        name: &str,
    ) -> bool {
        self.fields.contains_key(name)
    }

    pub fn add_method(
        &mut self,
        name: impl Into<String>,
        function: FuncRef,
    ) {
        self.methods.insert(
            name.into(),
            function,
        );
    }

    pub fn get_method(
        &self,
        name: &str,
    ) -> Option<FuncRef> {
        self.methods.get(name).cloned()
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Object")
            .field("fields", &self.fields)
            .field("methods", &self.methods.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<object>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Value;

    #[test]
    fn object_fields() {
        let object = Object::new();

        let object_ref = Rc::new(
            RefCell::new(object)
        );

        object_ref
            .borrow_mut()
            .set_field(
                "x",
                Value::Int(10)
            );

        assert_eq!(
            object_ref
                .borrow()
                .get_field("x"),
            Some(Value::Int(10))
        );
    }
}