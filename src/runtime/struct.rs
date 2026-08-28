use super::Value;

use std::{
    fmt,
    rc::Rc,
};

#[derive(Clone)]
pub struct StructType {
    name: String,
    fields: Vec<String>,
}

pub type StructTypeRef = Rc<StructType>;

impl StructType {
    pub fn new(
        name: impl Into<String>,
        fields: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    pub fn field_index(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.fields
            .iter()
            .position(|field| field == name)
    }
}

impl fmt::Display for StructType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "<struct {}>",
            self.name
        )
    }
}

#[derive(Clone)]
pub struct StructValue {
    ty: StructTypeRef,
    fields: Vec<Value>,
}

pub type StructValueRef = Rc<StructValue>;

impl StructValue {
    pub fn new(
        ty: StructTypeRef,
        fields: Vec<Value>,
    ) -> Result<Self, String> {
        if fields.len()
            != ty.fields().len()
        {
            return Err(
                format!(
                    "struct '{}' expects {} fields, got {}",
                    ty.name(),
                    ty.fields().len(),
                    fields.len(),
                )
            );
        }

        Ok(Self {
            ty,
            fields,
        })
    }

    pub fn ty(&self) -> StructTypeRef {
        self.ty.clone()
    }

    pub fn type_name(&self) -> &str {
        self.ty.name()
    }

    pub fn fields(&self) -> &[Value] {
        &self.fields
    }

    pub fn field(
        &self,
        index: usize,
    ) -> Option<Value> {
        self.fields
            .get(index)
            .cloned()
    }

    pub fn get_field(
        &self,
        name: &str,
    ) -> Option<Value> {
        let index =
            self.ty.field_index(name)?;

        self.field(index)
    }
}

impl fmt::Display for StructValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{} {{",
            self.type_name()
        )?;

        for (
            index,
            name,
        ) in self.ty.fields().iter().enumerate()
        {
            if index > 0 {
                write!(f, ", ")?;
            }

            let value =
                self.fields
                    .get(index)
                    .expect(
                        "StructValue field count invariant violated"
                    );

            write!(
                f,
                "{}: {}",
                name,
                value,
            )?;
        }

        write!(f, "}}")
    }
}