use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    runtime::{
        ExtensionHost,
        ExtensionRegistry,
        IteratorObj,
        IteratorRef,
        List,
        PathRef,
        ReceiverKind,
        SeriesRef,
        DataFrameRef,
        Value,
        ClosureRef,
        IterResult,
    },
    syntax::BinOp,
};

use std::{
    cell::RefCell,
    rc::Rc,
};

fn expect_arity(
    name: &str,
    args: &[Value],
    expected: usize,
) -> Result<()> {
    if args.len() != expected {
        return Err(
            Error::new(
                ErrorKind::Arity,
                format!(
                    "{}() expects {} argument(s), got {}",
                    name,
                    expected,
                    args.len(),
                ),
                None,
            )
        );
    }

    Ok(())
}

fn expect_int_arg(
    name: &str,
    value: &Value,
) -> Result<i64> {
    match value {
        Value::Int(value) =>
            Ok(*value),

        other =>
            Err(
                Error::new(
                    ErrorKind::Type,
                    format!(
                        "{}() expects Int, got {}",
                        name,
                        other.type_name(),
                    ),
                    None,
                )
            ),
    }
}

fn expect_closure_arg(
    name: &str,
    args: &[Value],
) -> Result<ClosureRef> {
    let value =
        args.get(0).ok_or_else(|| {
            Error::new(
                ErrorKind::Arity,
                format!(
                    "{}() missing closure argument",
                    name
                ),
                None,
            )
        })?;

    match value {
        Value::Closure(closure) =>
            Ok(closure.clone()),

        other =>
            Err(
                Error::new(
                    ErrorKind::Type,
                    format!(
                        "{}() expects a function, got {}",
                        name,
                        other.type_name(),
                    ),
                    None,
                )
            ),
    }
}

fn expect_closure_arg_at(
    name: &str,
    args: &[Value],
    index: usize,
) -> Result<ClosureRef> {
    let value =
        args.get(index).ok_or_else(|| {
            Error::new(
                ErrorKind::Arity,
                format!(
                    "{}() missing closure argument",
                    name
                ),
                None,
            )
        })?;

    match value {
        Value::Closure(closure) =>
            Ok(closure.clone()),

        other =>
            Err(
                Error::new(
                    ErrorKind::Type,
                    format!(
                        "{}() expects a function, got {}",
                        name,
                        other.type_name(),
                    ),
                    None,
                )
            ),
    }
}

//============================
// Registration
//============================
pub fn register_native_extensions(
    registry: &mut ExtensionRegistry,
) {
    register_string_extensions(
        registry
    );

    register_list_extensions(
        registry
    );

    register_series_extensions(
        registry
    );

    register_dataframe_extensions(
        registry
    );

    register_iterator_extensions(
        registry
    );

    register_range_extensions(
        registry
    );

    register_path_extensions(
        registry
    );
}

fn register_string_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::Str,
        "chars",
        string_chars,
    );

    registry.register_native(
        ReceiverKind::Str,
        "len",
        string_len,
    );

    registry.register_native(
        ReceiverKind::Str,
        "trim",
        string_trim,
    );

    registry.register_native(
        ReceiverKind::Str,
        "to_upper",
        string_to_upper,
    );

    registry.register_native(
        ReceiverKind::Str,
        "to_lower",
        string_to_lower,
    );

    registry.register_native(
        ReceiverKind::Str,
        "contains",
        string_contains,
    );

    registry.register_native(
        ReceiverKind::Str,
        "starts_with",
        string_starts_with,
    );

    registry.register_native(
        ReceiverKind::Str,
        "ends_with",
        string_ends_with,
    );

    registry.register_native(
        ReceiverKind::Str,
        "split",
        string_split,
    );

    registry.register_native(
        ReceiverKind::Str,
        "replace",
        string_replace,
    );

    registry.register_native(
        ReceiverKind::Str,
        "repeat",
        string_repeat,
    );
}

fn register_list_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::List,
        "len",
        list_len,
    );

    registry.register_native(
        ReceiverKind::List,
        "push",
        list_push,
    );

    registry.register_native(
        ReceiverKind::List,
        "iter",
        list_iter,
    );

    registry.register_native(
        ReceiverKind::List,
        "map",
        list_map,
    );

    registry.register_native(
        ReceiverKind::List,
        "filter",
        list_filter,
    );

    registry.register_native(
        ReceiverKind::List,
        "enumerate",
        list_enumerate,
    );

    registry.register_native(
        ReceiverKind::List,
        "zip",
        list_zip,
    );

    registry.register_native(
        ReceiverKind::List,
        "take",
        list_take,
    );

    registry.register_native(
        ReceiverKind::List,
        "skip",
        list_skip,
    );

    registry.register_native(
        ReceiverKind::List,
        "collect",
        list_collect,
    );

    registry.register_native(
        ReceiverKind::List,
        "reduce",
        list_reduce,
    );

    registry.register_native(
        ReceiverKind::List,
        "fold",
        list_fold,
    );

    registry.register_native(
        ReceiverKind::List,
        "any",
        list_any,
    );

    registry.register_native(
        ReceiverKind::List,
        "all",
        list_all,
    );

    registry.register_native(
        ReceiverKind::List,
        "sum",
        list_sum,
    );

    registry.register_native(
        ReceiverKind::List,
        "product",
        list_product,
    );

    registry.register_native(
        ReceiverKind::List,
        "min",
        list_min,
    );

    registry.register_native(
        ReceiverKind::List,
        "max",
        list_max,
    );
}

fn register_series_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::Series,
        "is_null",
        series_is_null,
    );

    registry.register_native(
        ReceiverKind::Series,
        "is_not_null",
        series_is_not_null,
    );

    registry.register_native(
        ReceiverKind::Series,
        "dropna",
        series_dropna,
    );

    registry.register_native(
        ReceiverKind::Series,
        "unique",
        series_unique,
    );

    registry.register_native(
        ReceiverKind::Series,
        "with_name",
        series_with_name,
    );

    registry.register_native(
        ReceiverKind::Series,
        "to_matrix",
        series_to_matrix,
    );

    registry.register_native(
        ReceiverKind::Series,
        "iter",
        series_iter,
    );
}

fn register_dataframe_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::DataFrame,
        "column",
        dataframe_column,
    );

    registry.register_native(
        ReceiverKind::DataFrame,
        "row",
        dataframe_row,
    );

    registry.register_native(
        ReceiverKind::DataFrame,
        "take_rows",
        dataframe_take_rows,
    );

    registry.register_native(
        ReceiverKind::DataFrame,
        "head",
        dataframe_head,
    );

    registry.register_native(
        ReceiverKind::DataFrame,
        "to_matrix",
        dataframe_to_matrix,
    );

    registry.register_native(
        ReceiverKind::DataFrame,
        "iter",
        dataframe_iter,
    );
}

fn register_iterator_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::Iterator,
        "next",
        iterator_next,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "map",
        iterator_map,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "filter",
        iterator_filter,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "enumerate",
        iterator_enumerate,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "zip",
        iterator_zip,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "take",
        iterator_take,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "skip",
        iterator_skip,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "collect",
        iterator_collect,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "reduce",
        iterator_reduce,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "fold",
        iterator_fold,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "any",
        iterator_any,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "all",
        iterator_all,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "sum",
        iterator_sum,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "product",
        iterator_product,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "min",
        iterator_min,
    );

    registry.register_native(
        ReceiverKind::Iterator,
        "max",
        iterator_max,
    );
}

fn register_path_extensions(
    registry: &mut ExtensionRegistry,
) {
    registry.register_native(
        ReceiverKind::Path,
        "name",
        path_name,
    );

    registry.register_native(
        ReceiverKind::Path,
        "extension",
        path_extension,
    );

    registry.register_native(
        ReceiverKind::Path,
        "stem",
        path_stem,
    );

    registry.register_native(
        ReceiverKind::Path,
        "parent",
        path_parent,
    );

    registry.register_native(
        ReceiverKind::Path,
        "join",
        path_join,
    );

    registry.register_native(
        ReceiverKind::Path,
        "exists",
        path_exists,
    );

    registry.register_native(
        ReceiverKind::Path,
        "is_file",
        path_is_file,
    );

    registry.register_native(
        ReceiverKind::Path,
        "is_dir",
        path_is_dir,
    );

    registry.register_native(
        ReceiverKind::Path,
        "string",
        path_string,
    );
}


//============================
// String
//============================
fn string_len(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!("Str extension received non-Str receiver");
    };

    expect_arity(
        "len",
        &args,
        0,
    )?;

    Ok(
        Value::Int(
            string.chars().count() as i64
        )
    )
}

fn string_trim(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "trim",
        &args,
        0,
    )?;

    Ok(
        Value::Str(
            Rc::new(
                string.trim().to_owned()
            )
        )
    )
}

fn string_to_upper(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "to_upper",
        &args,
        0,
    )?;

    Ok(
        Value::Str(
            Rc::new(
                string.to_uppercase()
            )
        )
    )
}

fn string_to_lower(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "to_lower",
        &args,
        0,
    )?;

    Ok(
        Value::Str(
            Rc::new(
                string.to_lowercase()
            )
        )
    )
}

fn string_contains(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "contains",
        &args,
        1,
    )?;

    let Value::Str(needle) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "contains() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    Ok(
        Value::Bool(
            string.contains(
                needle.as_str()
            )
        )
    )
}

fn string_starts_with(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "starts_with",
        &args,
        1,
    )?;

    let Value::Str(prefix) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "starts_with() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    Ok(
        Value::Bool(
            string.starts_with(
                prefix.as_str()
            )
        )
    )
}

fn string_ends_with(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "ends_with",
        &args,
        1,
    )?;

    let Value::Str(suffix) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "ends_with() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    Ok(
        Value::Bool(
            string.ends_with(
                suffix.as_str()
            )
        )
    )
}

fn string_chars(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "chars",
        &args,
        0,
    )?;

    Ok(
        Value::Iterator(
            Rc::new(
                RefCell::new(
                    IteratorObj::Str {
                        data: string,
                        byte_index: 0,
                    }
                )
            )
        )
    )
}

fn string_split(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "split",
        &args,
        1,
    )?;

    let Value::Str(separator) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "split() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    let values =
        string
            .split(separator.as_str())
            .map(|part| {
                Value::Str(
                    Rc::new(
                        part.to_owned()
                    )
                )
            })
            .collect();

    Ok(
        Value::List(
            List::new(values)
        )
    )
}

fn string_replace(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "replace",
        &args,
        2,
    )?;

    let Value::Str(from) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "replace() expects Str as first argument, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    let Value::Str(to) =
        &args[1]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "replace() expects Str as second argument, got {}",
                    args[1].type_name()
                ),
                None,
            )
        );
    };

    Ok(
        Value::Str(
            Rc::new(
                string.replace(
                    from.as_str(),
                    to.as_str(),
                )
            )
        )
    )
}

fn string_repeat(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Str(string) = receiver else {
        unreachable!();
    };

    expect_arity(
        "repeat",
        &args,
        1,
    )?;

    let count =
        expect_int_arg(
            "repeat",
            &args[0],
        )?;

    if count < 0 {
        return Err(
            Error::new(
                ErrorKind::Value,
                "repeat() does not accept negative counts",
                None,
            )
        );
    }

    Ok(
        Value::Str(
            Rc::new(
                string.repeat(
                    count as usize
                )
            )
        )
    )
}


//============================
// List
//============================
fn list_iterator(
    list: List,
) -> Result<IteratorRef> {
    IteratorObj::from_value(
        Value::List(list)
    )
    .map_err(|message| {
        Error::new(
            ErrorKind::Type,
            message,
            None,
        )
    })
}

fn list_len(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity(
        "len",
        &args,
        0,
    )?;

    Ok(
        Value::Int(
            list.len() as i64
        )
    )
}

fn list_push(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity(
        "push",
        &args,
        1,
    )?;

    list.push(
        args[0].clone()
    );

    Ok(Value::Unit)
}

fn list_iter(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity(
        "iter",
        &args,
        0,
    )?;

    Ok(
        Value::Iterator(
            list_iterator(list)?
        )
    )
}

fn list_map(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    let closure =
        expect_closure_arg(
            "map",
            &args,
        )?;

    let source =
        list_iterator(list)?;

    Ok(
        Value::Iterator(
            Rc::new(
                RefCell::new(
                    IteratorObj::Map {
                        source,
                        function: closure,
                    }
                )
            )
        )
    )
}


//============================
// Series
//============================
fn series_is_null(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity(
        "is_null",
        &args,
        0,
    )?;

    Ok(
        Value::Series(
            Rc::new(
                series.is_null()
            )
        )
    )
}

fn series_unique(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity(
        "unique",
        &args,
        0,
    )?;

    let value =
        series
            .unique()
            .map_err(|message| {
                Error::new(
                    ErrorKind::Runtime,
                    message,
                    None,
                )
            })?;

    Ok(
        Value::Series(
            Rc::new(value)
        )
    )
}

fn series_with_name(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity(
        "with_name",
        &args,
        1,
    )?;

    let Value::Str(name) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "with_name() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    Ok(
        Value::Series(
            Rc::new(
                series.with_name(
                    name.as_str()
                )
            )
        )
    )
}

fn series_to_matrix(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity(
        "to_matrix",
        &args,
        0,
    )?;

    let matrix =
        series
            .to_matrix()
            .map_err(|message| {
                Error::new(
                    ErrorKind::Type,
                    message,
                    None,
                )
            })?;

    Ok(
        Value::Matrix(
            Rc::new(
                RefCell::new(matrix)
            )
        )
    )
}


//============================
// DataFrame
//============================
fn dataframe_column(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity(
        "column",
        &args,
        1,
    )?;

    let Value::Str(name) =
        &args[0]
    else {
        return Err(
            Error::new(
                ErrorKind::Type,
                format!(
                    "column() expects Str, got {}",
                    args[0].type_name()
                ),
                None,
            )
        );
    };

    let column =
        df.column(
            name.as_str()
        )
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Name,
                format!(
                    "unknown DataFrame column '{}'",
                    name
                ),
                None,
            )
        })?;

    Ok(
        Value::Series(column)
    )
}

fn dataframe_row(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity(
        "row",
        &args,
        1,
    )?;

    let index =
        match &args[0] {
            Value::Int(index)
                if *index >= 0 =>
                *index as usize,

            Value::Int(_) =>
                return Err(
                    Error::new(
                        ErrorKind::Index,
                        "row() index must be non-negative",
                        None,
                    )
                ),

            other =>
                return Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "row() expects Int, got {}",
                            other.type_name()
                        ),
                        None,
                    )
                ),
        };

    let row =
        df.row(index)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Index,
                    format!(
                        "DataFrame row index out of bounds: {}",
                        index
                    ),
                    None,
                )
            })?;

    Ok(
        Value::Dict(row)
    )
}


//============================
// Iterator
//============================
fn iterator_next(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "next",
        &args,
        0,
    )?;

    match host.iterator_next(iterator)? {
        IterResult::Item(value) => {
            Ok(
                Value::Tuple(
                    Rc::new(
                        vec![
                            value,
                            Value::Bool(true),
                        ]
                    )
                )
            )
        }

        IterResult::End => {
            Ok(
                Value::Tuple(
                    Rc::new(
                        vec![
                            Value::Unit,
                            Value::Bool(false),
                        ]
                    )
                )
            )
        }
    }
}

fn iterator_map(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    let closure =
        expect_closure_arg(
            "map",
            &args,
        )?;

    Ok(
        Value::Iterator(
            Rc::new(
                RefCell::new(
                    IteratorObj::Map {
                        source: iterator,
                        function: closure,
                    }
                )
            )
        )
    )
}

fn iterator_reduce(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    let closure =
        expect_closure_arg(
            "reduce",
            &args,
        )?;

    host.reduce_iterator(
        iterator,
        closure,
    )
}

fn iterator_sum(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "sum",
        &args,
        0,
    )?;

    host.numeric_reduce(
        iterator,
        BinOp::Add,
    )
}

fn iterator_min(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "min",
        &args,
        0,
    )?;

    host.extreme_iterator(
        iterator,
        false,
    )
}

fn iterator_max(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "min",
        &args,
        0,
    )?;

    host.extreme_iterator(
        iterator,
        true,
    )
}


//============================
// Range
//============================
fn range_iterator(
    receiver: Value,
) -> Result<IteratorRef> {
    let Value::Range(
        start,
        end,
        inclusive,
    ) = receiver
    else {
        unreachable!();
    };

    IteratorObj::from_value(
        Value::Range(
            start,
            end,
            inclusive,
        )
    )
    .map_err(|message| {
        Error::new(
            ErrorKind::Type,
            message,
            None,
        )
    })
}

fn range_iter(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    expect_arity(
        "iter",
        &args,
        0,
    )?;

    Ok(
        Value::Iterator(
            range_iterator(receiver)?
        )
    )
}

fn range_map(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    let iterator =
        range_iterator(receiver)?;

    iterator_map(
        host,
        Value::Iterator(iterator),
        args,
        names,
    )
}


//============================
// Path
//============================
fn path_exists(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "exists",
        &args,
        0,
    )?;

    Ok(
        Value::Bool(
            path.exists()
        )
    )
}

fn path_join(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) =
        receiver
    else {
        unreachable!();
    };

    expect_arity(
        "join",
        &args,
        1,
    )?;

    let child =
        match &args[0] {
            Value::Str(value) =>
                std::path::PathBuf::from(
                    value.as_ref()
                ),

            Value::Path(value) =>
                value.to_path_buf(),

            other =>
                return Err(
                    Error::new(
                        ErrorKind::Type,
                        format!(
                            "join() expects Str or Path, got {}",
                            other.type_name()
                        ),
                        None,
                    )
                ),
        };

    Ok(
        Value::Path(
            Rc::new(
                path.join(&child)
            )
        )
    )
}




