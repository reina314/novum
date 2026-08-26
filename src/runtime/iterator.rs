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
                match data.get(*index) {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(value)
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

            Self::Vector {
                data,
                index,
            } => {
                Err(
                    "Vector iterator requires VM callback execution"
                        .into()
                )

                // match data.get(*index) {
                //     Some(value) => {
                //         *index += 1;

                //         Ok(
                //             IterResult::Item(
                //                 value
                //             )
                //         )
                //     }

                //     None =>
                //         Ok(
                //             IterResult::End
                //         ),
                // }
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

            Self::Enumerate {
                source,
                index,
            } => {
                let item =
                    Self::next(source)?;

                match item {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        let current =
                            *index;

                        *index += 1;

                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(
                                        vec![
                                            Value::Int(
                                                current as i64
                                            ),
                                            value,
                                        ]
                                    )
                                )
                            )
                        )
                    }
                }
            }

            Self::Zip {
                left,
                right,
            } => {
                let left_value =
                    Self::next(left)?;

                let right_value =
                    Self::next(right)?;

                match (
                    left_value,
                    right_value,
                ) {
                    (
                        IterResult::Item(left),
                        IterResult::Item(right),
                    ) => {
                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(
                                        vec![
                                            left,
                                            right,
                                        ]
                                    )
                                )
                            )
                        )
                    }

                    _ =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            Self::Take {
                source,
                remaining,
            } => {
                if *remaining == 0 {
                    return Ok(
                        IterResult::End
                    );
                }

                match Self::next(source)? {
                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),

                    IterResult::Item(value) => {
                        *remaining -= 1;

                        Ok(
                            IterResult::Item(
                                value
                            )
                        )
                    }
                }
            }

            Self::Skip {
                source,
                remaining,
            } => {
                while *remaining > 0 {
                    match Self::next(source)? {
                        IterResult::End =>
                            return Ok(
                                IterResult::End
                            ),

                        IterResult::Item(_) => {
                            *remaining -= 1;
                        }
                    }
                }

                Self::next(source)
            }

            Self::Map { .. } =>
                Err(
                    "Map iterator requires VM callback execution"
                        .into()
                ),

            Self::Filter { .. } =>
                Err(
                    "Filter iterator requires VM callback execution"
                        .into()
                ),
        }
    }
}

