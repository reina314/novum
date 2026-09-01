use crate::{
    error::{Error, ErrorKind, Result},
    runtime::{
        ClosureRef, ExtensionHost, ExtensionRegistry, GroupedDataFrame, IterResult, IteratorObj,
        IteratorRef, List, NativeExtensionFn, ReceiverKind, Value,
    },
    syntax::BinOp,
};

use std::{cell::RefCell, rc::Rc};

fn expect_arity(name: &str, args: &[Value], expected: usize) -> Result<()> {
    if args.len() != expected {
        return Err(Error::new(
            ErrorKind::Arity,
            format!(
                "{}() expects {} argument(s), got {}",
                name,
                expected,
                args.len(),
            ),
            None,
        ));
    }

    Ok(())
}

fn expect_int_arg(name: &str, value: &Value) -> Result<i64> {
    match value {
        Value::Int(value) => Ok(*value),

        other => Err(Error::new(
            ErrorKind::Type,
            format!("{}() expects Int, got {}", name, other.type_name(),),
            None,
        )),
    }
}

fn expect_closure_arg(name: &str, args: &[Value]) -> Result<ClosureRef> {
    let value = args.get(0).ok_or_else(|| {
        Error::new(
            ErrorKind::Arity,
            format!("{}() missing closure argument", name),
            None,
        )
    })?;

    match value {
        Value::Closure(closure) => Ok(closure.clone()),

        other => Err(Error::new(
            ErrorKind::Type,
            format!("{}() expects a function, got {}", name, other.type_name(),),
            None,
        )),
    }
}

fn expect_closure_arg_at(name: &str, args: &[Value], index: usize) -> Result<ClosureRef> {
    let value = args.get(index).ok_or_else(|| {
        Error::new(
            ErrorKind::Arity,
            format!("{}() missing closure argument", name),
            None,
        )
    })?;

    match value {
        Value::Closure(closure) => Ok(closure.clone()),

        other => Err(Error::new(
            ErrorKind::Type,
            format!("{}() expects a function, got {}", name, other.type_name(),),
            None,
        )),
    }
}

fn expect_usize_index(name: &str, value: &Value) -> Result<usize> {
    match value {
        Value::Int(index) if *index >= 0 => Ok(*index as usize),

        Value::Int(_) => Err(Error::new(
            ErrorKind::Index,
            format!("{}() index must be non-negative", name),
            None,
        )),

        other => Err(Error::new(
            ErrorKind::Type,
            format!("{}() expects Int, got {}", name, other.type_name()),
            None,
        )),
    }
}

fn expect_usize_indices(name: &str, value: &Value) -> Result<Vec<usize>> {
    let Value::List(list) = value else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("{}() expects List[Int]", name),
            None,
        ));
    };

    let values = list.as_vec();

    let mut result = Vec::with_capacity(values.len());

    for value in values.iter() {
        match value {
            Value::Int(index) if *index >= 0 => result.push(*index as usize),

            Value::Int(_) => {
                return Err(Error::new(
                    ErrorKind::Index,
                    format!("{}() index must be non-negative", name),
                    None,
                ))
            },

            other => {
                return Err(Error::new(
                    ErrorKind::Type,
                    format!("{}() expects List[Int], found {}", name, other.type_name()),
                    None,
                ))
            },
        }
    }

    Ok(result)
}

//============================
// Registration
//============================
pub fn register_native_extensions(registry: &mut ExtensionRegistry) {
    register_string_extensions(registry);

    register_list_extensions(registry);

    register_series_extensions(registry);

    register_dataframe_extensions(registry);

    register_grouped_dataframe_extensions(registry);

    register_iterator_extensions(registry);

    register_range_extensions(registry);

    register_path_extensions(registry);
}

fn register_string_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::Str, "chars", string_chars);

    registry.register_native(ReceiverKind::Str, "len", string_len);

    registry.register_native(ReceiverKind::Str, "trim", string_trim);

    registry.register_native(ReceiverKind::Str, "to_upper", string_to_upper);

    registry.register_native(ReceiverKind::Str, "to_lower", string_to_lower);

    registry.register_native(ReceiverKind::Str, "contains", string_contains);

    registry.register_native(ReceiverKind::Str, "starts_with", string_starts_with);

    registry.register_native(ReceiverKind::Str, "ends_with", string_ends_with);

    registry.register_native(ReceiverKind::Str, "split", string_split);

    registry.register_native(ReceiverKind::Str, "replace", string_replace);

    registry.register_native(ReceiverKind::Str, "repeat", string_repeat);
}

fn register_list_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::List, "len", list_len);

    registry.register_native(ReceiverKind::List, "push", list_push);

    registry.register_native(ReceiverKind::List, "iter", list_iter);

    registry.register_native(ReceiverKind::List, "map", list_map);

    registry.register_native(ReceiverKind::List, "filter", list_filter);

    registry.register_native(ReceiverKind::List, "enumerate", list_enumerate);

    registry.register_native(ReceiverKind::List, "zip", list_zip);

    registry.register_native(ReceiverKind::List, "take", list_take);

    registry.register_native(ReceiverKind::List, "skip", list_skip);

    registry.register_native(ReceiverKind::List, "collect", list_collect);

    registry.register_native(ReceiverKind::List, "reduce", list_reduce);

    registry.register_native(ReceiverKind::List, "fold", list_fold);

    registry.register_native(ReceiverKind::List, "any", list_any);

    registry.register_native(ReceiverKind::List, "all", list_all);

    registry.register_native(ReceiverKind::List, "sum", list_sum);

    registry.register_native(ReceiverKind::List, "product", list_product);

    registry.register_native(ReceiverKind::List, "min", list_min);

    registry.register_native(ReceiverKind::List, "max", list_max);
}

fn register_series_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::Series, "is_null", series_is_null);

    registry.register_native(ReceiverKind::Series, "is_not_null", series_is_not_null);

    registry.register_native(ReceiverKind::Series, "dropna", series_dropna);

    registry.register_native(ReceiverKind::Series, "unique", series_unique);

    registry.register_native(ReceiverKind::Series, "with_name", series_with_name);

    registry.register_native(ReceiverKind::Series, "to_matrix", series_to_matrix);

    registry.register_native(ReceiverKind::Series, "iter", series_iter);
}

fn register_dataframe_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::DataFrame, "column", dataframe_column);

    registry.register_native(ReceiverKind::DataFrame, "row", dataframe_row);

    registry.register_native(ReceiverKind::DataFrame, "take_rows", dataframe_take_rows);

    registry.register_native(ReceiverKind::DataFrame, "head", dataframe_head);

    registry.register_native(ReceiverKind::DataFrame, "tail", dataframe_tail);

    registry.register_native(ReceiverKind::DataFrame, "to_matrix", dataframe_to_matrix);

    registry.register_native(ReceiverKind::DataFrame, "iter", dataframe_iter);

    registry.register_native(ReceiverKind::DataFrame, "filter", dataframe_filter);

    registry.register_native(ReceiverKind::DataFrame, "group_by", dataframe_group_by);
}

fn register_grouped_dataframe_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(
        ReceiverKind::GroupedDataFrame,
        "aggregate",
        grouped_dataframe_aggregate,
    );

    registry.register_native(
        ReceiverKind::GroupedDataFrame,
        "count",
        grouped_dataframe_count,
    );

    registry.register_native(
        ReceiverKind::GroupedDataFrame,
        "mean",
        grouped_dataframe_mean,
    );

    registry.register_native(ReceiverKind::GroupedDataFrame, "sum", grouped_dataframe_sum);
}

fn register_iterator_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::Iterator, "next", iterator_next);

    registry.register_native(ReceiverKind::Iterator, "map", iterator_map);

    registry.register_native(ReceiverKind::Iterator, "filter", iterator_filter);

    registry.register_native(ReceiverKind::Iterator, "enumerate", iterator_enumerate);

    registry.register_native(ReceiverKind::Iterator, "zip", iterator_zip);

    registry.register_native(ReceiverKind::Iterator, "take", iterator_take);

    registry.register_native(ReceiverKind::Iterator, "skip", iterator_skip);

    registry.register_native(ReceiverKind::Iterator, "collect", iterator_collect);

    registry.register_native(ReceiverKind::Iterator, "reduce", iterator_reduce);

    registry.register_native(ReceiverKind::Iterator, "fold", iterator_fold);

    registry.register_native(ReceiverKind::Iterator, "any", iterator_any);

    registry.register_native(ReceiverKind::Iterator, "all", iterator_all);

    registry.register_native(ReceiverKind::Iterator, "sum", iterator_sum);

    registry.register_native(ReceiverKind::Iterator, "product", iterator_product);

    registry.register_native(ReceiverKind::Iterator, "min", iterator_min);

    registry.register_native(ReceiverKind::Iterator, "max", iterator_max);
}

fn register_range_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::Range, "iter", range_iter);

    registry.register_native(ReceiverKind::Range, "next", range_next);

    registry.register_native(ReceiverKind::Range, "map", range_map);

    registry.register_native(ReceiverKind::Range, "filter", range_filter);

    registry.register_native(ReceiverKind::Range, "enumerate", range_enumerate);

    registry.register_native(ReceiverKind::Range, "zip", range_zip);

    registry.register_native(ReceiverKind::Range, "take", range_take);

    registry.register_native(ReceiverKind::Range, "skip", range_skip);

    registry.register_native(ReceiverKind::Range, "collect", range_collect);

    registry.register_native(ReceiverKind::Range, "reduce", range_reduce);

    registry.register_native(ReceiverKind::Range, "fold", range_fold);

    registry.register_native(ReceiverKind::Range, "any", range_any);

    registry.register_native(ReceiverKind::Range, "all", range_all);

    registry.register_native(ReceiverKind::Range, "sum", range_sum);

    registry.register_native(ReceiverKind::Range, "product", range_product);

    registry.register_native(ReceiverKind::Range, "min", range_min);

    registry.register_native(ReceiverKind::Range, "max", range_max);
}

fn register_path_extensions(registry: &mut ExtensionRegistry) {
    registry.register_native(ReceiverKind::Path, "name", path_name);

    registry.register_native(ReceiverKind::Path, "extension", path_extension);

    registry.register_native(ReceiverKind::Path, "stem", path_stem);

    registry.register_native(ReceiverKind::Path, "parent", path_parent);

    registry.register_native(ReceiverKind::Path, "join", path_join);

    registry.register_native(ReceiverKind::Path, "exists", path_exists);

    registry.register_native(ReceiverKind::Path, "is_file", path_is_file);

    registry.register_native(ReceiverKind::Path, "is_dir", path_is_dir);

    registry.register_native(ReceiverKind::Path, "string", path_string);
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

    expect_arity("len", &args, 0)?;

    Ok(Value::Int(string.chars().count() as i64))
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

    expect_arity("trim", &args, 0)?;

    Ok(Value::Str(Rc::new(string.trim().to_owned())))
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

    expect_arity("to_upper", &args, 0)?;

    Ok(Value::Str(Rc::new(string.to_uppercase())))
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

    expect_arity("to_lower", &args, 0)?;

    Ok(Value::Str(Rc::new(string.to_lowercase())))
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

    expect_arity("contains", &args, 1)?;

    let Value::Str(needle) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("contains() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    Ok(Value::Bool(string.contains(needle.as_str())))
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

    expect_arity("starts_with", &args, 1)?;

    let Value::Str(prefix) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("starts_with() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    Ok(Value::Bool(string.starts_with(prefix.as_str())))
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

    expect_arity("ends_with", &args, 1)?;

    let Value::Str(suffix) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("ends_with() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    Ok(Value::Bool(string.ends_with(suffix.as_str())))
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

    expect_arity("chars", &args, 0)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Str {
        data: string,
        byte_index: 0,
    }))))
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

    expect_arity("split", &args, 1)?;

    let Value::Str(separator) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("split() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    let values = string
        .split(separator.as_str())
        .map(|part| Value::Str(Rc::new(part.to_owned())))
        .collect();

    Ok(Value::List(List::new(values)))
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

    expect_arity("replace", &args, 2)?;

    let Value::Str(from) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!(
                "replace() expects Str as first argument, got {}",
                args[0].type_name()
            ),
            None,
        ));
    };

    let Value::Str(to) = &args[1] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!(
                "replace() expects Str as second argument, got {}",
                args[1].type_name()
            ),
            None,
        ));
    };

    Ok(Value::Str(Rc::new(
        string.replace(from.as_str(), to.as_str()),
    )))
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

    expect_arity("repeat", &args, 1)?;

    let count = expect_int_arg("repeat", &args[0])?;

    if count < 0 {
        return Err(Error::new(
            ErrorKind::Value,
            "repeat() does not accept negative counts",
            None,
        ));
    }

    Ok(Value::Str(Rc::new(string.repeat(count as usize))))
}

//============================
// List
//============================
fn list_iterator(list: List) -> Result<IteratorRef> {
    IteratorObj::from_value(Value::List(list))
        .map_err(|message| Error::new(ErrorKind::Type, message, None))
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

    expect_arity("len", &args, 0)?;

    Ok(Value::Int(list.len() as i64))
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

    expect_arity("push", &args, 1)?;

    list.push(args[0].clone());

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

    expect_arity("iter", &args, 0)?;

    Ok(Value::Iterator(list_iterator(list)?))
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

    let closure = expect_closure_arg("map", &args)?;

    let source = list_iterator(list)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Map {
        source,
        function: closure,
    }))))
}

fn list_filter(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("filter", &args)?;

    let source = list_iterator(list)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorObj::Filter {
            source,
            predicate: closure,
        },
    ))))
}

fn list_enumerate(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("enumerate", &args, 0)?;

    let source = list_iterator(list)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorObj::Enumerate { source, index: 0 },
    ))))
}

fn list_zip(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("zip", &args, 1)?;

    let left = list_iterator(list)?;

    let right = match &args[0] {
        Value::Iterator(iterator) => iterator.clone(),

        value => IteratorObj::from_value(value.clone())
            .map_err(|message| Error::new(ErrorKind::Type, message, None))?,
    };

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Zip {
        left,
        right,
    }))))
}

fn list_take(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("take", &args, 1)?;

    let count = expect_int_arg("take", &args[0])?;

    if count < 0 {
        return Err(Error::new(
            ErrorKind::Value,
            "take() count must be non-negative",
            None,
        ));
    }

    let source = list_iterator(list)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Take {
        source,
        remaining: count as usize,
    }))))
}

fn list_skip(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("skip", &args, 1)?;

    let count = expect_int_arg("skip", &args[0])?;

    if count < 0 {
        return Err(Error::new(
            ErrorKind::Value,
            "skip() count must be non-negative",
            None,
        ));
    }

    let source = list_iterator(list)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Skip {
        source,
        remaining: count as usize,
    }))))
}

fn list_collect(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("collect", &args, 0)?;

    host.collect_iterator(list_iterator(list)?)
}

fn list_reduce(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("reduce", &args)?;

    host.reduce_iterator(list_iterator(list)?, closure)
}

fn list_fold(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("fold", &args, 2)?;

    let initial = args[0].clone();

    let closure = expect_closure_arg_at("fold", &args, 1)?;

    host.fold_iterator(list_iterator(list)?, initial, closure)
}

fn list_any(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("any", &args)?;

    host.any_iterator(list_iterator(list)?, closure)
}

fn list_all(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("all", &args)?;

    host.all_iterator(list_iterator(list)?, closure)
}

fn list_sum(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("sum", &args, 0)?;

    host.numeric_reduce(list_iterator(list)?, BinOp::Add)
}

fn list_product(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("product", &args, 0)?;

    host.numeric_reduce(list_iterator(list)?, BinOp::Mul)
}

fn list_min(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("min", &args, 0)?;

    host.extreme_iterator(list_iterator(list)?, false)
}

fn list_max(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::List(list) = receiver else {
        unreachable!();
    };

    expect_arity("max", &args, 0)?;

    host.extreme_iterator(list_iterator(list)?, true)
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

    expect_arity("is_null", &args, 0)?;

    Ok(Value::Series(Rc::new(series.is_null())))
}

fn series_is_not_null(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity("is_not_null", &args, 0)?;

    Ok(Value::Series(Rc::new(series.is_not_null())))
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

    expect_arity("unique", &args, 0)?;

    let value = series
        .unique()
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::Series(Rc::new(value)))
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

    expect_arity("with_name", &args, 1)?;

    let Value::Str(name) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("with_name() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    Ok(Value::Series(Rc::new(series.with_name(name.as_str()))))
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

    expect_arity("to_matrix", &args, 0)?;

    let matrix = series
        .to_matrix()
        .map_err(|message| Error::new(ErrorKind::Type, message, None))?;

    Ok(Value::Matrix(Rc::new(RefCell::new(matrix))))
}

fn series_dropna(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity("dropna", &args, 0)?;

    Ok(Value::Series(Rc::new(series.dropna())))
}

fn series_iter(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Series(series) = receiver else {
        unreachable!();
    };

    expect_arity("iter", &args, 0)?;

    let iterator = host.make_iterator(Value::Series(series))?;

    Ok(Value::Iterator(iterator))
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

    expect_arity("column", &args, 1)?;

    let Value::Str(name) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("column() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    let column = df.column(name.as_str()).ok_or_else(|| {
        Error::new(
            ErrorKind::Name,
            format!("unknown DataFrame column '{}'", name),
            None,
        )
    })?;

    Ok(Value::Series(column))
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

    expect_arity("row", &args, 1)?;

    let index = match &args[0] {
        Value::Int(index) if *index >= 0 => *index as usize,

        Value::Int(_) => {
            return Err(Error::new(
                ErrorKind::Index,
                "row() index must be non-negative",
                None,
            ))
        },

        other => {
            return Err(Error::new(
                ErrorKind::Type,
                format!("row() expects Int, got {}", other.type_name()),
                None,
            ))
        },
    };

    let row = df.row(index).ok_or_else(|| {
        Error::new(
            ErrorKind::Index,
            format!("DataFrame row index out of bounds: {}", index),
            None,
        )
    })?;

    Ok(Value::Dict(row))
}

fn dataframe_take_rows(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity("take_rows", &args, 1)?;

    let indices = expect_usize_indices("take_rows", &args[0])?;

    let result = df
        .take_rows(&indices)
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(result)))
}

fn dataframe_head(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    let n = match args.len() {
        0 => 5,

        1 => expect_usize_index("head", &args[0])?,

        _ => {
            return Err(Error::new(
                ErrorKind::Arity,
                format!("head() expects 0 or 1 argument(s), got {}", args.len()),
                None,
            ));
        },
    };

    let result = df
        .head(n)
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(result)))
}

fn dataframe_tail(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    let n = match args.len() {
        0 => 5,

        1 => expect_usize_index("tail", &args[0])?,

        _ => {
            return Err(Error::new(
                ErrorKind::Arity,
                format!("tail() expects 0 or 1 argument(s), got {}", args.len()),
                None,
            ));
        },
    };

    let result = df
        .tail(n)
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(result)))
}

fn dataframe_to_matrix(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity("to_matrix", &args, 0)?;

    let result = df
        .to_matrix()
        .map_err(|message| Error::new(ErrorKind::Type, message, None))?;

    Ok(Value::Matrix(Rc::new(RefCell::new(result))))
}

fn dataframe_iter(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity("iter", &args, 0)?;

    let iterator = host.make_iterator(Value::DataFrame(df))?;

    Ok(Value::Iterator(iterator))
}

fn dataframe_filter(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity("filter", &args, 1)?;

    let closure = expect_closure_arg("filter", &args)?;

    let mut keep = Vec::with_capacity(df.nrows());

    for index in 0..df.nrows() {
        let row = df.row(index).ok_or_else(|| {
            Error::new(
                ErrorKind::Index,
                format!("DataFrame row index out of bounds: {}", index),
                None,
            )
        })?;

        let result =
            host.call_closure_sync_named(closure.clone(), vec![Value::Dict(row)], &[None])?;

        let Value::Bool(value) = result else {
            return Err(Error::new(
                ErrorKind::Type,
                format!(
                    "DataFrame filter predicate must return Bool, got {}",
                    result.type_name()
                ),
                None,
            ));
        };

        keep.push(value);
    }

    let result = df
        .filter_rows(&keep)
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(result)))
}

fn dataframe_group_by(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::DataFrame(df) = receiver else {
        unreachable!();
    };

    expect_arity("group_by", &args, 1)?;

    let Value::Str(column) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("group_by() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    let grouped = GroupedDataFrame::from_columns(df, &[column.as_ref().clone()])
        .map_err(|message| Error::new(ErrorKind::Name, message, None))?;

    Ok(Value::GroupedDataFrame(Rc::new(grouped)))
}

//============================
// GroupedDataFrame
//============================
fn grouped_dataframe_aggregate(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::GroupedDataFrame(grouped) = receiver else {
        unreachable!();
    };

    expect_arity("aggregate", &args, 2)?;

    let Value::Str(column) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!(
                "aggregate() expects first argument as Str, got {}",
                args[0].type_name()
            ),
            None,
        ));
    };

    let Value::List(functions) = &args[1] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("aggregate() expects List, got {}", args[1].type_name()),
            None,
        ));
    };

    let mut names = Vec::with_capacity(functions.len());

    for value in functions.iter_cloned() {
        let Value::Str(name) = value else {
            return Err(Error::new(
                ErrorKind::Type,
                "aggregate() function names must be Str",
                None,
            ));
        };

        names.push(name.as_ref().clone());
    }

    let result = grouped
        .aggregate(column.as_str(), &names)
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(result)))
}

fn grouped_dataframe_count(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::GroupedDataFrame(grouped) = receiver else {
        unreachable!();
    };

    expect_arity("count", &args, 0)?;

    let dataframe = grouped
        .count()
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(dataframe)))
}

fn grouped_dataframe_mean(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::GroupedDataFrame(grouped) = receiver else {
        unreachable!();
    };

    expect_arity("mean", &args, 1)?;

    let Value::Str(column) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("mean() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    let dataframe = grouped
        .mean(column.as_str())
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(dataframe)))
}

fn grouped_dataframe_sum(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::GroupedDataFrame(grouped) = receiver else {
        unreachable!();
    };

    expect_arity("sum", &args, 1)?;

    let Value::Str(column) = &args[0] else {
        return Err(Error::new(
            ErrorKind::Type,
            format!("sum() expects Str, got {}", args[0].type_name()),
            None,
        ));
    };

    let dataframe = grouped
        .sum(column.as_str())
        .map_err(|message| Error::new(ErrorKind::Runtime, message, None))?;

    Ok(Value::DataFrame(Rc::new(dataframe)))
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
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("next", &args, 0)?;

    match host.iterator_next(iterator)? {
        IterResult::Item(value) => Ok(Value::Tuple(Rc::new(vec![value, Value::Bool(true)]))),

        IterResult::End => Ok(Value::Tuple(Rc::new(vec![Value::Unit, Value::Bool(false)]))),
    }
}

fn iterator_map(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("map", &args)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Map {
        source: iterator,
        function: closure,
    }))))
}

fn iterator_reduce(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("reduce", &args)?;

    host.reduce_iterator(iterator, closure)
}

fn iterator_sum(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("sum", &args, 0)?;

    host.numeric_reduce(iterator, BinOp::Add)
}

fn iterator_min(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("min", &args, 0)?;

    host.extreme_iterator(iterator, false)
}

fn iterator_max(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("max", &args, 0)?;

    host.extreme_iterator(iterator, true)
}

fn iterator_filter(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("filter", &args)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorObj::Filter {
            source: iterator,
            predicate: closure,
        },
    ))))
}

fn iterator_enumerate(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("enumerate", &args, 0)?;

    Ok(Value::Iterator(Rc::new(RefCell::new(
        IteratorObj::Enumerate {
            source: iterator,
            index: 0,
        },
    ))))
}

fn iterator_zip(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("zip", &args, 1)?;

    let other = match &args[0] {
        Value::Iterator(iterator) => iterator.clone(),

        value => IteratorObj::from_value(value.clone())
            .map_err(|message| Error::new(ErrorKind::Type, message, None))?,
    };

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Zip {
        left: iterator,
        right: other,
    }))))
}

fn iterator_take(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("take", &args, 1)?;

    let count = expect_int_arg("take", &args[0])?;

    if count < 0 {
        return Err(Error::new(
            ErrorKind::Value,
            "take() count must be non-negative",
            None,
        ));
    }

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Take {
        source: iterator,
        remaining: count as usize,
    }))))
}

fn iterator_skip(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("skip", &args, 1)?;

    let count = expect_int_arg("skip", &args[0])?;

    if count < 0 {
        return Err(Error::new(
            ErrorKind::Value,
            "skip() count must be non-negative",
            None,
        ));
    }

    Ok(Value::Iterator(Rc::new(RefCell::new(IteratorObj::Skip {
        source: iterator,
        remaining: count as usize,
    }))))
}

fn iterator_collect(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("collect", &args, 0)?;

    host.collect_iterator(iterator)
}

fn iterator_fold(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("fold", &args, 2)?;

    let initial = args[0].clone();

    let closure = expect_closure_arg_at("fold", &args, 1)?;

    host.fold_iterator(iterator, initial, closure)
}

fn iterator_any(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("any", &args)?;

    host.any_iterator(iterator, closure)
}

fn iterator_all(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    let closure = expect_closure_arg("all", &args)?;

    host.all_iterator(iterator, closure)
}

fn iterator_product(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Iterator(iterator) = receiver else {
        unreachable!();
    };

    expect_arity("product", &args, 0)?;

    host.numeric_reduce(iterator, BinOp::Mul)
}

//============================
// Range
//============================
fn range_iterator(receiver: Value) -> Result<IteratorRef> {
    let Value::Range(start, end, inclusive) = receiver else {
        unreachable!();
    };

    IteratorObj::from_value(Value::Range(start, end, inclusive))
        .map_err(|message| Error::new(ErrorKind::Type, message, None))
}

fn range_with_iterator(
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
    function: NativeExtensionFn,
    host: &mut dyn ExtensionHost,
) -> Result<Value> {
    let iterator = range_iterator(receiver)?;

    function(host, Value::Iterator(iterator), args, names)
}

fn range_iter(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    expect_arity("iter", &args, 0)?;

    Ok(Value::Iterator(range_iterator(receiver)?))
}

fn range_map(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    let iterator = range_iterator(receiver)?;

    iterator_map(host, Value::Iterator(iterator), args, names)
}

fn range_next(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_next, host)
}

fn range_filter(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_filter, host)
}

fn range_enumerate(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_enumerate, host)
}

fn range_zip(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_zip, host)
}

fn range_take(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_take, host)
}

fn range_skip(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_skip, host)
}

fn range_collect(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_collect, host)
}

fn range_reduce(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_reduce, host)
}

fn range_fold(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_fold, host)
}

fn range_any(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_any, host)
}

fn range_all(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_all, host)
}

fn range_sum(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_sum, host)
}

fn range_product(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_product, host)
}

fn range_min(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_min, host)
}

fn range_max(
    host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    names: &[Option<String>],
) -> Result<Value> {
    range_with_iterator(receiver, args, names, iterator_max, host)
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
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("exists", &args, 0)?;

    Ok(Value::Bool(path.exists()))
}

fn path_join(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("join", &args, 1)?;

    let child = match &args[0] {
        Value::Str(value) => std::path::PathBuf::from(value.as_ref()),

        Value::Path(value) => value.to_path_buf(),

        other => {
            return Err(Error::new(
                ErrorKind::Type,
                format!("join() expects Str or Path, got {}", other.type_name()),
                None,
            ))
        },
    };

    Ok(Value::Path(Rc::new(path.join(&child))))
}

fn path_name(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("name", &args, 0)?;

    match path.name() {
        Some(value) => Ok(crate::stdlib::option_some(Value::Str(Rc::new(value)))),

        None => Ok(crate::stdlib::option_none()),
    }
}

fn path_extension(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("extension", &args, 0)?;

    match path.extension() {
        Some(value) => Ok(crate::stdlib::option_some(Value::Str(Rc::new(value)))),

        None => Ok(crate::stdlib::option_none()),
    }
}

fn path_stem(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("stem", &args, 0)?;

    match path.stem() {
        Some(value) => Ok(crate::stdlib::option_some(Value::Str(Rc::new(value)))),

        None => Ok(crate::stdlib::option_none()),
    }
}

fn path_parent(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("parent", &args, 0)?;

    match path.parent() {
        Some(parent) => Ok(crate::stdlib::option_some(Value::Path(Rc::new(parent)))),

        None => Ok(crate::stdlib::option_none()),
    }
}

fn path_is_file(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("is_file", &args, 0)?;

    Ok(Value::Bool(path.is_file()))
}

fn path_is_dir(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("is_dir", &args, 0)?;

    Ok(Value::Bool(path.is_dir()))
}

fn path_string(
    _host: &mut dyn ExtensionHost,
    receiver: Value,
    args: Vec<Value>,
    _names: &[Option<String>],
) -> Result<Value> {
    let Value::Path(path) = receiver else {
        unreachable!();
    };

    expect_arity("string", &args, 0)?;

    Ok(Value::Str(Rc::new(path.to_string_lossy())))
}
