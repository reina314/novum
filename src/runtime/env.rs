use super::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug)]
struct EnvFrame {
    parent: Option<Env>,
    values: RefCell<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct Env(Rc<EnvFrame>);

impl Env {
    pub fn global() -> Self { Self::new(None) }

    pub fn new(parent: Option<Env>) -> Self {
        Self(Rc::new(EnvFrame { parent, values: RefCell::new(HashMap::new()) }))
    }

    pub fn child(&self) -> Self { Self::new(Some(self.clone())) }

    pub fn define(&self, name: impl Into<String>, value: Value) {
        self.0.values.borrow_mut().insert(name.into(), value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.0.values.borrow().get(name) { return Some(v.clone()); }
        self.0.parent.as_ref().and_then(|p| p.get(name))
    }

    pub fn assign(&self, name: &str, value: Value) -> bool {
        if self.0.values.borrow().contains_key(name) {
            self.0.values.borrow_mut().insert(name.to_owned(), value);
            true
        } else if let Some(parent) = &self.0.parent {
            parent.assign(name, value)
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

    pub fn contains_local(&self, name: &str) -> bool {
        self.0.values.borrow().contains_key(name)
    }

    pub fn remove_local(&self, name: &str) -> Option<Value> {
        self.0.values.borrow_mut().remove(name)
    }
}
