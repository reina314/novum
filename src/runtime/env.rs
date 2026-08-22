use super::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug)]
struct EnvFrame {
    parent: Option<Env>,
    values: RefCell<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct Env(Rc<EnvFrame>);

/// TODO: Migrate all language-level declaration sites from define to declare, then consider removing the legacy API.
impl Env {
    pub fn global() -> Self {
        Self::new(None)
    }

    pub fn new(parent: Option<Env>) -> Self {
        Self(
            Rc::new(
                EnvFrame {
                    parent,
                    values: RefCell::new(
                        HashMap::new()
                    ),
                }
            )
        )
    }

    pub fn child(&self) -> Self {
        Self::new(
            Some(self.clone())
        )
    }

    pub fn assign_or_define(
        &self,
        name: impl Into<String>,
        value: Value,
    ) {
        let name = name.into();

        // If the name already exists somewhere in the
        // lexical environment chain, update the nearest binding.
        if self.assign(
            &name,
            value.clone(),
        ) {
            return;
        }

        // Otherwise create a new binding in the current scope.
        self.define(
            name,
            value,
        );
    }

    /// Legacy API.
    /// raw insertion
    pub fn define(
        &self,
        name: impl Into<String>,
        value: Value,
    ) {
        self.0
            .values
            .borrow_mut()
            .insert(
                name.into(),
                value,
            );
    }

    /// New API.
    /// explicit declaration with duplication check
    pub fn declare(
        &self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<(), String> {
        let mut bindings =
            HashMap::new();

        bindings.insert(
            name.into(),
            value,
        );

        self.declare_all(
            bindings
        )
    }

    /// New API: atomic multi-binding declaration.
    pub fn declare_all(
        &self,
        bindings: HashMap<String, Value>,
    ) -> Result<(), String> {
        let mut values =
            self.0.values.borrow_mut();

        for name in bindings.keys() {
            if values.contains_key(name) {
                return Err(format!(
                    "variable '{}' is already defined in the current scope",
                    name
                ));
            }
        }

        values.extend(bindings);

        Ok(())
    }

    pub fn get(
        &self,
        name: &str,
    ) -> Option<Value> {
        if let Some(value) =
            self.0
                .values
                .borrow()
                .get(name)
        {
            return Some(value.clone());
        }

        self.0
            .parent
            .as_ref()
            .and_then(|parent| {
                parent.get(name)
            })
    }

    /// Legacy API: recursive assignment.
    pub fn assign(
        &self,
        name: &str,
        value: Value,
    ) -> bool {
        if self
            .0
            .values
            .borrow()
            .contains_key(name)
        {
            self.0
                .values
                .borrow_mut()
                .insert(
                    name.to_owned(),
                    value,
                );

            true
        } else if let Some(parent) =
            &self.0.parent
        {
            parent.assign(
                name,
                value,
            )
        } else {
            false
        }
    }

    /// New API: current scope only.
    pub fn assign_local(
        &self,
        name: &str,
        value: Value,
    ) -> bool {
        let mut values =
            self.0.values.borrow_mut();

        if values.contains_key(name) {
            values.insert(
                name.to_owned(),
                value,
            );

            true
        } else {
            false
        }
    }

    pub fn local_values(
        &self,
    ) -> HashMap<String, Value> {
        self.0
            .values
            .borrow()
            .clone()
    }

    pub fn contains_local(
        &self,
        name: &str,
    ) -> bool {
        self.0
            .values
            .borrow()
            .contains_key(name)
    }

    pub fn remove_local(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.0
            .values
            .borrow_mut()
            .remove(name)
    }
}