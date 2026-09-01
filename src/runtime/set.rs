use std::{cell::RefCell, rc::Rc};

use crate::runtime::Value;

pub type SetRef = Rc<RefCell<Set>>;

#[derive(Clone)]
pub struct Set {
    values: Vec<Value>,
}

impl Set {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn from_values(values: Vec<Value>) -> Result<Self, String> {
        let mut set = Self::new();

        for value in values {
            set.add(value)?;
        }

        Ok(set)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn contains(&self, value: &Value) -> Result<bool, String> {
        for existing in &self.values {
            if Value::eq_values(existing, value)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn add(&mut self, value: Value) -> Result<(), String> {
        if !self.contains(&value)? {
            self.values.push(value);
        }

        Ok(())
    }

    pub fn remove(&mut self, value: &Value) -> Result<bool, String> {
        let position = self
            .values
            .iter()
            .position(|existing| Value::eq_values(existing, value).unwrap_or(false));

        match position {
            Some(index) => {
                self.values.remove(index);
                Ok(true)
            },

            None => Ok(false),
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    pub fn union(&self, other: &Self) -> Result<Self, String> {
        let mut result = Self::from_values(self.values.clone())?;

        for value in &other.values {
            result.add(value.clone())?;
        }

        Ok(result)
    }

    pub fn intersection(&self, other: &Self) -> Result<Self, String> {
        let mut result = Self::new();

        for value in &self.values {
            if other.contains(value)? {
                result.add(value.clone())?;
            }
        }

        Ok(result)
    }

    pub fn difference(&self, other: &Self) -> Result<Self, String> {
        let mut result = Self::new();

        for value in &self.values {
            if !other.contains(value)? {
                result.add(value.clone())?;
            }
        }

        Ok(result)
    }
}
