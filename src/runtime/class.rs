use super::{
    ClosureRef,
    Object,
    ObjectRef,
};

use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
    cell::RefCell,
};

pub type ClassRef = Rc<Class>;

pub struct FieldDefinition {
    name: String,
    default: Option<ClosureRef>,
}

impl FieldDefinition {
    pub fn new(
        name: impl Into<String>,
        default: Option<ClosureRef>,
    ) -> Self {
        Self {
            name: name.into(),
            default,
        }
    }

    pub fn name(
        &self,
    ) -> &str {
        &self.name
    }

    pub fn default(
        &self,
    ) -> Option<ClosureRef> {
        self.default.clone()
    }
}

pub struct Class {
    name: String,
    fields: Vec<FieldDefinition>,
    methods: HashMap<String, ClosureRef>,
}

impl Class {
    pub fn new(
        name: impl Into<String>,
        fields: Vec<FieldDefinition>,
        methods: HashMap<String, ClosureRef>,
    ) -> Self {
        Self {
            name: name.into(),
            fields,
            methods,
        }
    }

    pub fn name(
        &self,
    ) -> &str {
        &self.name
    }

    pub fn fields(
        &self,
    ) -> &[FieldDefinition] {
        &self.fields
    }

    pub fn field(
        &self,
        name: &str,
    ) -> Option<&FieldDefinition> {
        self.fields
            .iter()
            .find(
                |field|
                    field.name() == name
            )
    }

    pub fn method(
        &self,
        name: &str,
    ) -> Option<ClosureRef> {
        self.methods
            .get(name)
            .cloned()
    }

    pub fn constructor(
        &self,
    ) -> Option<ClosureRef> {
        self.methods
            .get("init")
            .cloned()
    }

    pub fn instantiate(
        self: &ClassRef,
    ) -> ObjectRef {
        Rc::new(
            RefCell::new(
                Object::new(
                    self.clone()
                )
            )
        )
    }

}

impl fmt::Debug for Class {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Class")
            .field(
                "name",
                &self.name,
            )
            .field(
                "fields",
                &self.fields
                    .iter()
                    .map(
                        FieldDefinition::name
                    )
                    .collect::<Vec<_>>(),
            )
            .field(
                "methods",
                &self.methods
                    .keys()
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl fmt::Display for Class {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<class {}>",
            self.name
        )
    }
}