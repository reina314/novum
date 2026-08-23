use super::{
    FuncRef,
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

pub struct Class {
    name: String,
    constructor: Option<FuncRef>,
    methods: HashMap<String, FuncRef>,
}

impl Class {
    pub fn new(
        name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            constructor: None,
            methods:
                HashMap::new(),
        }
    }

    pub fn name(
        &self,
    ) -> &str {
        &self.name
    }

    pub fn set_constructor(
        &mut self,
        function: FuncRef,
    ) {
        self.constructor =
            Some(function);
    }

    pub fn constructor(
        &self,
    ) -> Option<FuncRef> {
        self.constructor
            .clone()
    }

    pub fn add_method(
        &mut self,
        name: impl Into<String>,
        function: FuncRef,
    ) {
        self.methods.insert(
            name.into(),
            function,
        );
    }

    pub fn get_method(
        &self,
        name: &str,
    ) -> Option<FuncRef> {
        self.methods
            .get(name)
            .cloned()
    }

    pub fn has_method(
        &self,
        name: &str,
    ) -> bool {
        self.methods.contains_key(
            name
        )
    }

    pub fn instantiate(
        self: &ClassRef,
    ) -> ObjectRef {
        Rc::new(
            RefCell::new(
                Object::with_class(
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
                "constructor",
                &self.constructor
                    .is_some(),
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