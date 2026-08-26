use super::{
    ClassRef,
    Value,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type ObjectRef = Rc<RefCell<Object>>;

#[derive(Clone)]
pub struct Object {
    class: Option<ClassRef>,
    fields: HashMap<String, Value>,
}

impl Object {
    pub fn new() -> Self {
        Self {
            class: None,
            fields:
                HashMap::new(),
        }
    }

    pub fn with_class(
        class: ClassRef,
    ) -> Self {
        Self {
            class: Some(class),
            fields:
                HashMap::new(),
        }
    }

    pub fn class(
        &self,
    ) -> Option<ClassRef> {
        self.class.clone()
    }

    pub fn type_name(
        &self,
    ) -> &str {
        match &self.class {
            Some(class) =>
                class.name(),

            None =>
                "Object",
        }
    }

    pub fn get_field(
        &self,
        name: &str,
    ) -> Option<Value> {
        self.fields
            .get(name)
            .cloned()
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
        self.fields.contains_key(
            name
        )
    }

    pub fn get_method(
        &self,
        name: &str,
    ) -> Option<FuncRef> {
        self.class
            .as_ref()
            .and_then(|class| {
                class.get_method(
                    name
                )
            })
    }

    pub fn fmt_display(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{} {{",
            self.type_name()
        )?;

        let mut fields =
            self.fields
                .iter()
                .collect::<Vec<_>>();

        fields.sort_by(
            |(a, _), (b, _)|
                a.cmp(b)
        );

        for (
            index,
            (name, value)
        ) in fields.iter().enumerate()
        {
            if index > 0 {
                write!(f, ", ")?;
            }

            write!(
                f,
                "{}: {}",
                name,
                value
            )?;
        }

        write!(f, "}}")
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Object {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct(
            self.type_name()
        )
        .field(
            "fields",
            &self.fields
        )
        .finish()
    }
}