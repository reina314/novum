use super::Value;

use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
};

pub type EnumRef = Rc<EnumDef>;

#[derive(Clone)]
pub struct EnumDef {
    name: String,
    variants: HashMap<String, EnumVariant>,
}

#[derive(Clone)]
pub struct EnumVariant {
    name: String,
    arity: usize,
}

impl fmt::Debug for EnumVariant {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}({} args)",
            self.name,
            self.arity
        )
    }
}

impl EnumDef {
    pub fn new(
        name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            variants: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_variant(
        &mut self,
        name: impl Into<String>,
        arity: usize,
    ) -> Result<(), String> {
        let name =
            name.into();

        if self.variants.contains_key(&name) {
            return Err(format!(
                "duplicate enum variant '{}'",
                name
            ));
        }

        self.variants.insert(
            name.clone(),
            EnumVariant {
                name,
                arity,
            },
        );

        Ok(())
    }

    pub fn variant(
        &self,
        name: &str,
    ) -> Option<&EnumVariant> {
        self.variants.get(name)
    }

    pub fn variants(
        &self,
    ) -> &HashMap<String, EnumVariant> {
        &self.variants
    }
}

impl EnumVariant {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arity(&self) -> usize {
        self.arity
    }
}

impl fmt::Debug for EnumDef {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<enum {}>",
            self.name
        )
    }
}

impl fmt::Display for EnumDef {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<enum {}>",
            self.name
        )
    }
}

#[derive(Clone)]
pub struct EnumValue {
    enum_name: String,
    variant: String,
    fields: Vec<Value>,
}

pub type EnumValueRef =
    Rc<EnumValue>;

impl EnumValue {
    pub fn new(
        enum_name: impl Into<String>,
        variant: impl Into<String>,
        fields: Vec<Value>,
    ) -> Self {
        Self {
            enum_name: enum_name.into(),
            variant: variant.into(),
            fields,
        }
    }

    pub fn enum_name(&self) -> &str {
        &self.enum_name
    }

    pub fn variant(&self) -> &str {
        &self.variant
    }

    pub fn fields(&self) -> &[Value] {
        &self.fields
    }

    pub fn field(
        &self,
        index: usize,
    ) -> Option<Value> {
        self.fields.get(index).cloned()
    }
}

impl fmt::Debug for EnumValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.enum_name,
            self.variant,
        )?;

        if !self.fields.is_empty() {
            write!(f, "(")?;

            for (i, value)
                in self.fields.iter().enumerate()
            {
                if i > 0 {
                    write!(f, ", ")?;
                }

                write!(
                    f,
                    "{:?}",
                    value
                )?;
            }

            write!(f, ")")?;
        }

        Ok(())
    }
}

impl fmt::Display for EnumValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.enum_name,
            self.variant,
        )?;

        if !self.fields.is_empty() {
            write!(f, "(")?;

            for (i, value)
                in self.fields.iter().enumerate()
            {
                if i > 0 {
                    write!(f, ", ")?;
                }

                write!(
                    f,
                    "{}",
                    value
                )?;
            }

            write!(f, ")")?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct EnumConstructor {
    enum_def: EnumRef,
    variant: String,
}

impl EnumConstructor {
    pub fn new(
        enum_def: EnumRef,
        variant: impl Into<String>,
    ) -> Self {
        Self {
            enum_def,
            variant: variant.into(),
        }
    }

    pub fn enum_def(
        &self,
    ) -> &EnumRef {
        &self.enum_def
    }

    pub fn variant(&self) -> &str {
        &self.variant
    }
}

impl fmt::Debug for EnumConstructor {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let variant =
            self.enum_def
                .variant(&self.variant);

        match variant {
            Some(variant) => {
                write!(
                    f,
                    "<enum constructor {}.{} / {} args>",
                    self.enum_def.name(),
                    self.variant,
                    variant.arity(),
                )
            }

            None => {
                write!(
                    f,
                    "<enum constructor {}.{}>",
                    self.enum_def.name(),
                    self.variant,
                )
            }
        }
    }
}

impl fmt::Display for EnumConstructor {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.enum_def.name(),
            self.variant
        )
    }
}