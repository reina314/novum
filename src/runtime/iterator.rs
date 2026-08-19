use super::{List, Value};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum IteratorObj {
    List { data: List, index: usize },
    Str { data: Rc<Vec<char>>, index: usize },
    Range { current: i64, end: i64 },
}

impl IteratorObj {
    pub fn next(&mut self) -> Option<Value> {
        match self {
            Self::List { data, index } => {
                let data = data.borrow();
                let value = data.get(*index)?.clone();
                *index += 1;
                Some(value)
            }
            Self::Str { data, index } => {
                let ch = *data.get(*index)?;
                *index += 1;
                Some(Value::Str(Rc::new(ch.to_string())))
            }
            Self::Range { current, end } => {
                if *current >= *end { None } else {
                    let value = *current;
                    *current += 1;
                    Some(Value::Int(value))
                }
            }
        }
    }
}
