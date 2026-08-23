use super::{
    FuncRef,
    Class,
    ClassRef,
    ObjectRef,
};
use std::{
    fmt,
    rc::Rc,
    collections::HashMap,
};

pub type StructRef = Rc<StructDefinition>;

pub struct StructDefinition {
    name: String,
    fields: Vec<String>,
    methods: HashMap<String, FuncRef>,
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

    pub fn to_class(
        &self,
    ) -> ClassRef {
        let mut class =
            Class::new(
                self.name.clone()
            );

        for (name, function)
            in &self.methods
        {
            if name == "init" {
                class.set_constructor(
                    function.clone()
                );
            } else {
                class.add_method(
                    name.clone(),
                    function.clone(),
                );
            }
        }

        Rc::new(class)
    }

    pub fn instantiate(
        &self,
    ) -> Result<ObjectRef, String> {
        let class =
            self.to_class();

        Ok(
            class.instantiate()
        )
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