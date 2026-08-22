use super::{
    List,
    Value,
    FuncRef,
};
use std::{
    rc::Rc,
    cell::RefCell,
};

pub type IteratorRef = Rc<RefCell<IteratorObj>>;

#[derive(Clone)]
pub enum IteratorObj {
    List {
        data: List,
        index: usize
    },

    Str {
        data: Rc<Vec<char>>,
        index: usize
    },

    Range {
        current: i64,
        end: i64
    },

    Map {
        source: IteratorRef,
        function: FuncRef,
    },

    Filter {
        source: IteratorRef,
        predicate: FuncRef,
    },

    Enumerate {
        source: IteratorRef,
        index: usize,
    },

    Zip {
        left: IteratorRef,
        right: IteratorRef,
    },

    Take {
        source: IteratorRef,
        remaining: usize,
    },

    Skip {
        source: IteratorRef,
        remaining: usize,
    },
}

impl IteratorObj {
    pub fn from_value(
        value: Value,
    ) -> Result<IteratorRef, String> {
        match value {
            Value::Iterator(iterator) => {
                Ok(iterator)
            }

            Value::List(data) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data,
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Str(string) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Str {
                                data: Rc::new(
                                    string
                                        .chars()
                                        .collect()
                                ),
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Range(
                start,
                end,
                inclusive,
            ) => {
                let end =
                    if inclusive {
                        end.checked_add(1)
                            .ok_or_else(|| {
                                "inclusive range endpoint overflow"
                                    .to_owned()
                            })?
                    } else {
                        end
                    };

                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Range {
                                current: start,
                                end,
                            }
                        )
                    )
                )
            }

            other => {
                Err(
                    format!(
                        "{} is not iterable",
                        other.type_name()
                    )
                )
            }
        }
    }
}


