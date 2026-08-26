use super::{
    List,
    ListRef,
    Value,
    StrRef,
    VectorRef,
};
use crate::vm::ClosureRef;
use std::{
    rc::Rc,
    cell::RefCell,
};

pub type IteratorRef = Rc<RefCell<IteratorObj>>;

#[derive(Clone)]
pub enum IteratorObj {
    List {
        data: ListRef,
        index: usize
    },

    Str {
        data: StrRef,
        byte_index: usize
    },

    Vector {
        data: VectorRef,
        index: usize,
    },

    Range {
        current: i64,
        end: i64
    },

    Map {
        source: IteratorRef,
        function: ClosureRef,
    },

    Filter {
        source: IteratorRef,
        predicate: ClosureRef,
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

pub enum IterResult {
    Item(Value),
    End,
}

impl IteratorObj {
    pub fn from_value(
        value: Value,
    ) -> Result<IteratorRef, String> {
        match value {
            Value::Iterator(iterator) =>
                Ok(iterator),

            Value::List(data) =>
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data,
                                index: 0,
                            }
                        )
                    )
                ),

            Value::Str(data) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Str {
                                data,
                                byte_index: 0,
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

            // Dict → items()
            Value::Dict(dict) => {
                let items =
                    dict.borrow()
                        .iter()
                        .map(|(key, value)| {
                            Value::Tuple(
                                Rc::new(vec![
                                    Value::Str(
                                        Rc::new(key.clone())
                                    ),
                                    value.clone(),
                                ])
                            )
                        })
                        .collect::<Vec<_>>();

                let list =
                    Rc::new(
                        List::new(
                            items
                        )
                    );

                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data: list,
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Set(set) => {
                let values =
                    set.borrow()
                        .values()
                        .to_vec();

                let list =
                    Rc::new(
                        List::new(
                            values
                        )
                    );

                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::List {
                                data: list,
                                index: 0,
                            }
                        )
                    )
                )
            }

            Value::Vector(vector) => {
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Vector {
                                data: vector,
                                index: 0,
                            }
                        )
                    )
                )
            }

            other =>
                Err(
                    format!(
                        "{} is not iterable",
                        other.type_name()
                    )
                ),
        }
    }

    pub fn next(
        iterator: &IteratorRef,
    ) -> Result<IterResult, String> {
        let mut iterator =
            iterator.borrow_mut();

        match &mut *iterator {
            Self::List {
                data,
                index,
            } => {
                let value =
                    data.get(*index);

                match value {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            Self::Str {
                data,
                byte_index,
            } => {
                let slice =
                    &data[*byte_index..];

                let Some(ch) =
                    slice.chars().next()
                else {
                    return Ok(
                        IterResult::End
                    );
                };

                *byte_index +=
                    ch.len_utf8();

                Ok(
                    IterResult::Item(
                        Value::Str(
                            Rc::new(
                                ch.to_string()
                            )
                        )
                    )
                )
            }

            Self::Range {
                current,
                end,
            } => {
                if *current >= *end {
                    return Ok(
                        IterResult::End
                    );
                }

                let value =
                    *current;

                *current += 1;

                Ok(
                    IterResult::Item(
                        Value::Int(value)
                    )
                )
            }

            Self::Vector {
                data,
                index,
            } => {
                Err(
                    "Vector iterator requires VM callback execution"
                        .into()
                )
            }

            Self::Map {
                ..
            } => {
                // Implement after VM callback invocation
                // is available.
                Err(
                    "Map iterator requires VM callback execution"
                        .into()
                )
            }

            Self::Filter {
                ..
            } => {
                Err(
                    "Filter iterator requires VM callback execution"
                        .into()
                )
            }

            Self::Enumerate {
                ..
            } |
            Self::Zip {
                ..
            } |
            Self::Take {
                ..
            } |
            Self::Skip {
                ..
            } => {
                Err(
                    "iterator variant is not implemented yet"
                        .into()
                )
            }
        }
    }
}

