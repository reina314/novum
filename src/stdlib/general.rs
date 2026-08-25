use crate::{
    runtime::{
        Value,
        EnumValue,
    },
    syntax::{
        Pattern,
    },
};

use std::{
    rc::Rc,
};

pub fn result_ok(value: Value) -> Value {
    // Use your existing Result constructor helper here
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Result",
                "Ok",
                vec![value],
            )
        )
    )
}

pub fn result_err(message: impl Into<String>) -> Value {
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Result",
                "Err",
                vec![
                    Value::Str(
                        Rc::new(
                            message.into()
                        )
                    )
                ],
            )
        )
    )
}

pub fn option_some(value: Value) -> Value {
    // Use your existing Option constructor helper here.
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "Some",
                vec![value],
            )
        )
    )
}

pub fn option_none() -> Value {
    // Use your existing Option constructor helper here.
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "None",
                vec![],
            )
        )
    )
}
