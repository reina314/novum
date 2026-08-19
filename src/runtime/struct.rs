use super::{FuncRef, Object, ObjectRef, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type StructRef = Rc<StructDefinition>;

pub struct StructDefinition {
    pub name: String,
    pub fields: Vec<String>,
    pub methods: HashMap<String, FuncRef>,
}

impl StructDefinition {
    pub fn new(
        name: impl Into<String>,
        fields: Vec<String>,
        methods: HashMap<String, FuncRef>,
    ) -> Self {
        Self {
            name: name.into(),
            fields,
            methods,
        }
    }

    pub fn instantiate(
        &self,
        args: Vec<Value>,
    ) -> Result<ObjectRef, String> {
        if args.len() != self.fields.len() {
            return Err(format!(
                "{} expects {} arguments, got {}",
                self.name,
                self.fields.len(),
                args.len()
            ));
        }

        let mut object = Object::new();

        for (field, value) in self.fields.iter().zip(args) {
            object.set_field(field.clone(), value);
        }

        for (name, function) in &self.methods {
            object.add_method(
                name.clone(),
                function.clone(),
            );
        }

        object.set_type_name(self.name.clone());

        Ok(Rc::new(
            RefCell::new(object)
        ))
    }
}

impl fmt::Debug for StructDefinition {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("StructDefinition")
            .field("name", &self.name)
            .field("fields", &self.fields)
            .field(
                "methods",
                &self.methods.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}