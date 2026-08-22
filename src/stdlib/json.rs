use std::{
    cell::RefCell,
    rc::Rc,
};

use serde_json::Value as JsonValue;

use crate::runtime::{
    Value,
    ModuleRef,
};


pub fn module() -> ModuleRef {
    let mut module =
        crate::runtime::Module::new("json");

    module.set_exported(
        "parse",
        Value::Builtin(
            parse
        ),
    );

    module.set_exported(
        "stringify",
        Value::Builtin(
            stringify
        ),
    );

    Rc::new(
        RefCell::new(
            module
        )
    )
}


pub fn parse(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "json.parse() expects exactly 1 argument"
                .into()
        );
    }

    let text =
        match &args[0] {
            Value::Str(text) =>
                text.as_str(),

            other => {
                return Err(
                    format!(
                        "json.parse() expects Str, got {}",
                        other.type_name()
                    )
                );
            }
        };

    let json =
        serde_json::from_str::<JsonValue>(
            text
        )
        .map_err(|error| {
            format!(
                "invalid JSON: {}",
                error
            )
        })?;

    from_json_value(
        json
    )
}

pub fn stringify(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "json.stringify() expects exactly 1 argument"
                .into()
        );
    }

    let json =
        to_json_value(
            &args[0]
        )?;

    let text =
        serde_json::to_string(
            &json
        )
        .map_err(|error| {
            format!(
                "failed to serialize JSON: {}",
                error
            )
        })?;

    Ok(
        Value::Str(
            Rc::new(text)
        )
    )
}

fn from_json_value(
    value: JsonValue,
) -> Result<Value, String> {
    match value {
        JsonValue::Null =>
            Ok(Value::Null),

        JsonValue::Bool(value) =>
            Ok(Value::Bool(value)),

        JsonValue::Number(number) => {
            if let Some(value) =
                number.as_i64()
            {
                Ok(
                    Value::Int(value)
                )
            } else if let Some(value) =
                number.as_f64()
            {
                Ok(
                    Value::Float(value)
                )
            } else {
                Err(
                    format!(
                        "unsupported JSON number: {}",
                        number
                    )
                )
            }
        }

        JsonValue::String(value) =>
            Ok(
                Value::Str(
                    Rc::new(value)
                )
            ),

        JsonValue::Array(values) => {
            let mut result =
                Vec::with_capacity(
                    values.len()
                );

            for value in values {
                result.push(
                    from_json_value(value)?
                );
            }

            Ok(
                Value::List(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        JsonValue::Object(values) => {
            let mut result =
                std::collections::HashMap::new();

            for (key, value) in values {
                result.insert(
                    key,
                    from_json_value(value)?,
                );
            }

            Ok(
                Value::Dict(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }
    }
}

fn to_json_value(
    value: &Value,
) -> Result<JsonValue, String> {
    match value {
        Value::Null =>
            Ok(JsonValue::Null),

        Value::Unit =>
            Ok(JsonValue::Null),

        Value::Bool(value) =>
            Ok(JsonValue::Bool(*value)),

        Value::Int(value) =>
            Ok(
                JsonValue::Number(
                    (*value).into()
                )
            ),

        Value::Float(value) => {
            let number =
                serde_json::Number::from_f64(
                    *value
                )
                .ok_or_else(|| {
                    format!(
                        "cannot serialize {} as JSON number",
                        value
                    )
                })?;

            Ok(
                JsonValue::Number(number)
            )
        }

        Value::Str(value) =>
            Ok(
                JsonValue::String(
                    (**value).clone()
                )
            ),

        Value::List(list) => {
            let list =
                list.borrow();

            let mut result =
                Vec::with_capacity(
                    list.len()
                );

            for value in list.iter() {
                result.push(
                    to_json_value(value)?
                );
            }

            Ok(
                JsonValue::Array(result)
            )
        }

        Value::Dict(dict) => {
            let dict =
                dict.borrow();

            let mut result =
                serde_json::Map::new();

            for (key, value)
                in dict.iter()
            {
                result.insert(
                    key.clone(),
                    to_json_value(value)?,
                );
            }

            Ok(
                JsonValue::Object(result)
            )
        }

        other => {
            Err(
                format!(
                    "{} cannot be serialized as JSON",
                    other.type_name()
                )
            )
        }
    }
}


