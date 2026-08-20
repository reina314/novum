use super::Value;

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type ModuleRef =
    Rc<RefCell<Module>>;

#[derive(Clone)]
pub struct Module {
    name: String,
    exports: HashMap<String, Value>,
}

impl Module {
    pub fn new(
        name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            exports: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set(
        &mut self,
        name: impl Into<String>,
        value: Value,
    ) {
        self.exports.insert(
            name.into(),
            value,
        );
    }

    pub fn get(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.exports
            .get(name)
            .cloned()
    }

    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains_key(name)
    }
}

impl fmt::Debug for Module {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Module")
            .field("name", &self.name)
            .field(
                "exports",
                &self.exports.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl fmt::Display for Module {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "<module {}>", self.name)
    }
}