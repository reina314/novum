use crate::{
    runtime::{
        Value,
        IteratorObj,
    }
};

use std::{
    // cell::RefCell,
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

