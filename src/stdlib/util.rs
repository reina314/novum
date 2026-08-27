use crate::{
    error::{
        Error,
        ErrorKind,
    },
    syntax::Pattern,
    runtime::{
        Value,
        EnumValue,
    },
};

use std::{
    rc::Rc,
};

#[inline]
pub fn is_self_pattern(
    pattern: &Pattern,
) -> bool {
    matches!(
        pattern,
        Pattern::Ident(name)
            if name == "self"
    )
}

pub fn encode_method_call(
    method_index: u16,
    argc: u16,
) -> u32 {
    ((method_index as u32) << 16)
        | argc as u32
}

pub fn decode_method_call(
    operand: u32,
) -> (u16, u16) {
    (
        (operand >> 16) as u16,
        (operand & 0xffff) as u16,
    )
}

pub fn encode_class_counts(
    fields: usize,
    methods: usize,
) -> u32 {
    ((fields as u32) << 16)
        | methods as u32
}

pub fn decode_class_counts(
    operand: u32,
) -> (usize, usize) {
    (
        (operand >> 16) as usize,
        (operand & 0xffff) as usize,
    )
}

pub fn encode_call_operand(
    call_site: usize,
    argc: usize,
) -> Result<u32, Error> {
    if call_site > 0xffff
        || argc > 0xffff
    {
        return Err(
            Error::new(
                ErrorKind::Runtime,
                "too many call arguments or call sites",
                None,
            )
        );
    }

    Ok(
        ((call_site as u32) << 16)
            | argc as u32
    )
}

pub fn decode_call_operand(
    operand: u32,
) -> (usize, usize) {
    (
        (operand >> 16) as usize,
        (operand & 0xffff) as usize,
    )
}

pub fn result_ok(
    value: Value,
) -> Value {
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

pub fn result_err(
    message: impl Into<String>,
) -> Value {
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

pub fn option_some(
    value: Value,
) -> Value {
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
    Value::EnumValue(
        Rc::new(
            EnumValue::new(
                "Option",
                "None",
                Vec::new(),
            )
        )
    )
}



