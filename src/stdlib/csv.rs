use crate::runtime::{
    DataFrame,
    Series,
    Value,
};

use std::{
    fs::File,
    rc::Rc,
};

fn parse_value(
    text: &str,
) -> Value {
    let trimmed =
        text.trim();

    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("na")
        || trimmed.eq_ignore_ascii_case("null")
    {
        return Value::Null;
    }

    if trimmed == "true" {
        return Value::Bool(true);
    }

    if trimmed == "false" {
        return Value::Bool(false);
    }

    if let Ok(v) =
        trimmed.parse::<i64>()
    {
        return Value::Int(v);
    }

    if let Ok(v) =
        trimmed.parse::<f64>()
    {
        return Value::Float(v);
    }

    Value::Str(
        Rc::new(
            trimmed.to_owned()
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
            .map_err(|e| {
                format!(
                    "failed to open '{}': {}",
                    path,
                    e
                )
            })?;

    let mut reader =
        csv::Reader::from_reader(file);

    let headers =
        reader.headers()
            .map_err(|e| {
                format!(
                    "failed to read CSV header: {}",
                    e
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
            record.map_err(|e| {
                format!(
                    "failed to read CSV record: {}",
                    e
                )
            })?;

        if record.len()
            != headers.len()
        {
            return Err(
                "CSV row has incorrect number of fields"
                    .into()
            );
        }

        for i in 0..record.len() {
            columns[i].push(
                parse_value(
                    &record[i]
                )
            );
        }
    }

    let series = columns
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

    let df =
        DataFrame::from_series(
            series
        )?;

    Ok(
        Value::DataFrame(
            Rc::new(df)
        )
    )
}