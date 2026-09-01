use super::{ClassRef, Value};

use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

pub type ObjectRef = Rc<RefCell<Object>>;

#[derive(Clone)]
pub struct Object {
    class: ClassRef,
    fields: HashMap<String, Value>,
}

impl Object {
    pub fn new(class: ClassRef) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }

    #[inline]
    pub fn class(&self) -> ClassRef {
        self.class.clone()
    }

    #[inline]
    pub fn type_name(&self) -> &str {
        self.class.name()
    }

    #[inline]
    pub fn get_field(&self, name: &str) -> Option<Value> {
        self.fields.get(name).cloned()
    }

    #[inline]
    pub fn set_field(&mut self, name: impl Into<String>, value: Value) {
        self.fields.insert(name.into(), value);
    }

    #[inline]
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(String::as_str)
    }

    pub fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {{", self.type_name())?;

        let mut fields = self.fields.iter().collect::<Vec<_>>();

        fields.sort_unstable_by_key(|(a, _)| *a);

        for (index, (name, value)) in fields.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }

            write!(f, "{}: {}", name, value)?;
        }

        write!(f, "}}")
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(self.type_name())
            .field("fields", &self.fields)
            .finish()
    }
}
