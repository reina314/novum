use crate::runtime::{
    DataFrame,
    Series,
    Value,
    Module,
    ModuleRef,
};

use std::{
    fs::File,
    rc::Rc,
    cell::RefCell,
};

pub fn module() -> ModuleRef {
    let mut module =
        Module::new("csv");

    module.set_exported(
        "read",
        Value::Builtin(read),
    );

    Rc::new(
        RefCell::new(module)
    )
}


fn parse_value(
    text: &str,
) -> Value {
    let text =
        text.trim();

    // Missing values
    if text.is_empty()
        || text.eq_ignore_ascii_case("na")
        || text.eq_ignore_ascii_case("null")
        || text == "."
    {
        return Value::Null;
    }

    // Bool
    if text.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }

    if text.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    // Int
    if let Ok(value) =
        text.parse::<i64>()
    {
        return Value::Int(value);
    }

    // Float
    if let Ok(value) =
        text.parse::<f64>()
    {
        return Value::Float(value);
    }

    // String
    Value::Str(
        Rc::new(
            text.to_owned()
        )
    )
}

pub fn read(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "csv.read() expects exactly 1 argument"
                .into()
        );
    }

    let path =
        match &args[0] {
            Value::Str(path) =>
                path.as_ref(),

            other => {
                return Err(format!(
                    "csv.read() expects Str, got {}",
                    other.type_name()
                ));
            }
        };

    let file =
        File::open(path)
            .map_err(|error| {
                format!(
                    "failed to open '{}': {}",
                    path,
                    error
                )
            })?;

    let mut reader =
        csv::Reader::from_reader(file);

    let headers =
        reader
            .headers()
            .map_err(|error| {
                format!(
                    "failed to read CSV header: {}",
                    error
                )
            })?
            .clone();

    if headers.is_empty() {
        return Err(
            "CSV must contain at least one column"
                .into()
        );
    }

    let mut columns =
        vec![
            Vec::<Value>::new();
            headers.len()
        ];

    for record in reader.records() {
        let record =
            record.map_err(|error| {
                format!(
                    "failed to read CSV record: {}",
                    error
                )
            })?;

        if record.len() != headers.len() {
            return Err(format!(
                "CSV row has {} fields, expected {}",
                record.len(),
                headers.len()
            ));
        }

        for i in 0..record.len() {
            columns[i].push(
                parse_value(
                    &record[i]
                )
            );
        }
    }

    let series =
        columns
            .into_iter()
            .enumerate()
            .map(|(i, data)| {
                Rc::new(
                    Series::new(
                        headers[i].to_owned(),
                        data,
                    )
                )
            })
            .collect::<Vec<_>>();

    let dataframe =
        DataFrame::from_series(
            series
        )?;

    Ok(
        Value::DataFrame(
            Rc::new(dataframe)
        )
    )
}