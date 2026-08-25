use super::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

#[derive(Debug)]
enum Bindings {
    Empty,

    One {
        name: String,
        value: Value,
    },

    Map(HashMap<String, Value>),
}

impl Default for Bindings {
    fn default() -> Self {
        Self::Empty
    }
}

impl Bindings {
    #[inline]
    fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Self::Empty => None,

            Self::One {
                name: key,
                value,
            } => {
                if key == name {
                    Some(value)
                } else {
                    None
                }
            }

            Self::Map(map) => {
                map.get(name)
            }
        }
    }

    #[inline]
    fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    #[inline]
    fn insert(
        &mut self,
        name: String,
        value: Value,
    ) {
        match self {
            Self::Empty => {
                *self =
                    Self::One {
                        name,
                        value,
                    };
            }

            Self::One {
                name: existing_name,
                value: existing_value,
            } => {
                if existing_name == &name {
                    *existing_value = value;
                } else {
                    let old_name =
                        std::mem::take(
                            existing_name
                        );

                    let old_value =
                        std::mem::replace(
                            existing_value,
                            Value::Unit,
                        );

                    let mut map =
                        HashMap::with_capacity(4);

                    map.insert(
                        old_name,
                        old_value,
                    );

                    map.insert(
                        name,
                        value,
                    );

                    *self =
                        Self::Map(map);
                }
            }

            Self::Map(map) => {
                map.insert(
                    name,
                    value,
                );
            }
        }
    }

    #[inline]
    fn get_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut Value> {
        match self {
            Self::Empty => None,

            Self::One {
                name: key,
                value,
            } => {
                if key == name {
                    Some(value)
                } else {
                    None
                }
            }

            Self::Map(map) => {
                map.get_mut(name)
            }
        }
    }

    fn remove(
        &mut self,
        name: &str,
    ) -> Option<Value> {
        match self {
            Self::Empty => None,

            Self::One {
                name: key,
                ..
            } if key == name => {
                let current =
                    std::mem::replace(
                        self,
                        Self::Empty,
                    );

                match current {
                    Self::One { value, .. } =>
                        Some(value),

                    _ =>
                        unreachable!(),
                }
            }

            Self::One { .. } =>
                None,

            Self::Map(map) =>
                map.remove(name),
        }
    }

    fn to_hash_map(
        &self,
    ) -> HashMap<String, Value> {
        match self {
            Self::Empty =>
                HashMap::new(),

            Self::One {
                name,
                value,
            } => {
                let mut map =
                    HashMap::with_capacity(1);

                map.insert(
                    name.clone(),
                    value.clone(),
                );

                map
            }

            Self::Map(map) =>
                map.clone(),
        }
    }

    fn extend(
        &mut self,
        bindings: HashMap<String, Value>,
    ) {
        if bindings.is_empty() {
            return;
        }

        if matches!(
            self,
            Self::Empty
        ) && bindings.len() == 1 {
            let mut iter =
                bindings.into_iter();

            let (
                name,
                value,
            ) = iter
                .next()
                .unwrap();

            *self =
                Self::One {
                    name,
                    value,
                };

            return;
        }

        match self {
            Self::Empty => {
                *self =
                    Self::Map(
                        bindings
                    );
            }

            Self::One {
                name,
                value,
            } => {
                let old_name =
                    std::mem::take(name);

                let old_value =
                    std::mem::replace(
                        value,
                        Value::Unit,
                    );

                let mut map =
                    HashMap::with_capacity(
                        bindings.len() + 1
                    );

                map.insert(
                    old_name,
                    old_value,
                );

                map.extend(
                    bindings
                );

                *self =
                    Self::Map(map);
            }

            Self::Map(map) => {
                map.extend(
                    bindings
                );
            }
        }
    }
}

#[derive(Debug)]
struct EnvFrame {
    parent: Option<Env>,
    values: RefCell<Bindings>,
}

#[derive(Debug, Clone)]
pub struct Env(Rc<EnvFrame>);

/// TODO: Migrate all language-level declaration sites from define to declare, then consider removing the legacy API.
impl Env {
    pub fn global() -> Self {
        Self::new(None)
    }

    pub fn new(
        parent: Option<Env>,
    ) -> Self {
        Self(
            Rc::new(
                EnvFrame {
                    parent,
                    values: RefCell::new(
                        Bindings::Empty
                    ),
                }
            )
        )
    }

    #[inline]
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
        let name =
            name.into();

        if self.assign(
            &name,
            value.clone(),
        ) {
            return;
        }

        self.define(
            name,
            value,
        );
    }

    #[inline]
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

    pub fn declare(
        &self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<(), String> {
        let name =
            name.into();

        let mut values =
            self.0
                .values
                .borrow_mut();

        if values.contains_key(
            &name
        ) {
            return Err(
                format!(
                    "variable '{}' is already defined in the current scope",
                    name
                )
            );
        }

        values.insert(
            name,
            value,
        );

        Ok(())
    }

    pub fn declare_all(
        &self,
        bindings: HashMap<String, Value>,
    ) -> Result<(), String> {
        let mut values =
            self.0
                .values
                .borrow_mut();

        for name in bindings.keys() {
            if values.contains_key(name) {
                return Err(
                    format!(
                        "variable '{}' is already defined in the current scope",
                        name
                    )
                );
            }
        }

        values.extend(
            bindings
        );

        Ok(())
    }

    #[inline]
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
            return Some(
                value.clone()
            );
        }

        self.0
            .parent
            .as_ref()
            .and_then(
                |parent| {
                    parent.get(name)
                }
            )
    }

    pub fn assign(
        &self,
        name: &str,
        value: Value,
    ) -> bool {
        {
            let mut values =
                self.0
                    .values
                    .borrow_mut();

            if let Some(slot) =
                values.get_mut(name)
            {
                *slot = value;

                return true;
            }
        }

        if let Some(parent) =
            &self.0.parent
        {
            return parent.assign(
                name,
                value,
            );
        }

        false
    }

    #[inline]
    pub fn assign_local(
        &self,
        name: &str,
        value: Value,
    ) -> bool {
        let mut values =
            self.0
                .values
                .borrow_mut();

        if let Some(slot) =
            values.get_mut(name)
        {
            *slot = value;

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
            .to_hash_map()
    }

    #[inline]
    pub fn contains_local(
        &self,
        name: &str,
    ) -> bool {
        self.0
            .values
            .borrow()
            .contains_key(name)
    }

    #[inline]
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