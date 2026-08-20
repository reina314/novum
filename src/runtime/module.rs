use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    path::PathBuf,
    rc::Rc,
};

use super::Value;

pub type ModuleRef =
    Rc<RefCell<Module>>;

#[derive(Clone, Debug)]
pub struct ModulePath {
    parts: Vec<String>,
}

impl ModulePath {
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }

    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    pub fn name(&self) -> String {
        self.parts.join(".")
    }

    pub fn last(&self) -> Option<&str> {
        self.parts
            .last()
            .map(String::as_str)
    }

    pub fn join(
        &self,
        other: &[String],
    ) -> Self {
        let mut parts =
            self.parts.clone();

        parts.extend(
            other.iter().cloned()
        );

        Self { parts }
    }
}

#[derive(Clone, Debug)]
pub struct ModuleContext {
    pub module_path: ModulePath,
    pub file_path: PathBuf,
}

impl ModuleContext {
    pub fn new(
        module_path: ModulePath,
        file_path: PathBuf,
    ) -> Self {
        Self {
            module_path,
            file_path,
        }
    }

    pub fn name(&self) -> String {
        self.module_path.name()
    }
}

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