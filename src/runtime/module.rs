use std::{
    fmt,
    rc::Rc,
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
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

impl fmt::Display for ModulePath {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug)]
pub struct ModuleContext {
    pub module_path: ModulePath,
    pub file_path: PathBuf,
    pub exports: HashSet<String>,
}

impl ModuleContext {
    pub fn new(
        module_path: ModulePath,
        file_path: PathBuf,
    ) -> Self {
        Self {
            module_path,
            file_path,
            exports: HashSet::new(),
        }
    }

    pub fn name(&self) -> String {
        self.module_path.name()
    }

    pub fn export(
        &mut self,
        name: impl Into<String>,
    ) {
        self.exports.insert(
            name.into()
        );
    }

    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }
}

#[derive(Clone)]
pub struct Module {
    name: String,
    fields: HashMap<String, Value>,
    exports: HashSet<String>,
}

impl Module {
    pub fn new(
        name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            fields: HashMap::new(),
            exports: HashSet::new(),
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
        self.fields.insert(
            name.into(),
            value,
        );
    }

    pub fn export(
        &mut self,
        name: impl Into<String>,
    ) {
        self.exports.insert(
            name.into()
        );
    }

    /// Set a field and make it publicly accessible.
    pub fn set_exported(
        &mut self,
        name: impl Into<String>,
        value: Value,
    ) {
        let name = name.into();

        self.fields.insert(
            name.clone(),
            value,
        );

        self.exports.insert(
            name,
        );
    }

    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }

    /// Alias for `get_internal()`
    pub fn get(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.fields
            .get(name)
            .cloned()
    }

    pub fn get_field(
        &self,
        name: &str,
    ) -> Option<Value> {
        if !self.is_exported(name) {
            return None;
        }

        self.fields
            .get(name)
            .cloned()
    }

    pub fn get_internal(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.fields
            .get(name)
            .cloned()
    }

    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.fields.contains_key(name)
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
                "fields",
                &self.fields.keys().collect::<Vec<_>>(),
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