use crate::{
    error::ErrorKind,
    runtime::{
        List,
        Value,
        IteratorObj,
    }
};

use std::{
    cell::RefCell,
    rc::Rc,
};

pub fn print(args: Vec<Value>) -> Result<Value,String> {
    for value in args { println!("{}", value); }
    Ok(Value::Unit)
}

pub fn iter(mut args: Vec<Value>) -> Result<Value,String> {
    if args.len()!=1 { return Err("iter() takes exactly 1 argument".into()); }
    match args.remove(0) {
        Value::Iterator(it)=>Ok(Value::Iterator(it)),
        Value::List(data)=>Ok(Value::Iterator(IteratorObj::List{data,index:0})),
        Value::Str(s)=>Ok(Value::Iterator(IteratorObj::Str{data:Rc::new(s.chars().collect()),index:0})),
        other=>Err(format!("{} is not iterable",other.type_name())),
    }
}

pub fn call_list_method(list: List, name: &str, mut args: Vec<Value>) -> Result<Value,(ErrorKind,String)> {
    match name {
        "push" => {
            if args.len()!=1 { return Err((ErrorKind::Arity,"push() takes exactly 1 argument".into())); }
            list.borrow_mut().push(args.remove(0));
            Ok(Value::Unit)
        }
        "pop" => Ok(list.borrow_mut().pop().unwrap_or(Value::Unit)),
        "remove" => {
            if args.len()!=1 { return Err((ErrorKind::Arity,"remove() takes exactly 1 argument".into())); }
            let index = match args.remove(0) { Value::Int(i) if i>=0 => i as usize, Value::Int(_) => return Err((ErrorKind::Index,"remove() does not accept negative indices".into())), other=>return Err((ErrorKind::Type,format!("remove() expects Int, got {}",other.type_name()))) };
            let mut list = list.borrow_mut();
            if index>=list.len() { return Err((ErrorKind::Index,format!("index out of range: {}",index))); }
            Ok(list.remove(index))
        }
        "len" => Ok(Value::Int(list.borrow().len() as i64)),
        _ => Err((ErrorKind::Runtime,format!("unknown list method: {}",name))),
    }
}

pub fn make_empty_list() -> Value { Value::List(Rc::new(RefCell::new(Vec::new()))) }
