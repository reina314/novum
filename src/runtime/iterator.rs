use super::{
    List,
    Value,
    StrRef,
    VectorRef,
    ClosureRef,
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
                    List::new(
                        items
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
                    List::new(
                        values
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
        let primitive =
            {
                let borrowed =
                    iterator.borrow();

                matches!(
                    &*borrowed,
                    Self::List { .. }
                    | Self::Str { .. }
                    | Self::Vector { .. }
                    | Self::Range { .. }
                )
            };

        if primitive {
            Self::next_primitive(
                iterator
            )
        } else {
            Self::next_composite(
                iterator
            )
        }
    }

    fn next_primitive(
        iterator: &IteratorRef,
    ) -> Result<IterResult, String> {
        let mut iterator =
            iterator.borrow_mut();

        match &mut *iterator {
            /*
            * ----------------------------------------------------
            * List
            * ----------------------------------------------------
            */
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

            /*
            * ----------------------------------------------------
            * String
            * ----------------------------------------------------
            */
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

            /*
            * ----------------------------------------------------
            * Vector
            * ----------------------------------------------------
            */
            Self::Vector {
                data,
                index,
            } => {
                let value =
                    data.borrow()
                        .get(*index);

                match value {
                    Some(value) => {
                        *index += 1;

                        Ok(
                            IterResult::Item(
                                Value::Float(
                                    value
                                )
                            )
                        )
                    }

                    None =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * ----------------------------------------------------
            * Range
            * ----------------------------------------------------
            *
            * This is the hot path for:
            *
            *     for i in 0..N { ... }
            */
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
                        Value::Int(
                            value
                        )
                    )
                )
            }

            _ => {
                unreachable!(
                    "next_primitive called for composite iterator"
                )
            }
        }
    }

    fn next_composite(
        iterator: &IteratorRef,
    ) -> Result<IterResult, String> {
        enum Composite {
            Map {
                function: ClosureRef,
            },

            Filter {
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

        let composite =
            {
                let borrowed =
                    iterator.borrow();

                match &*borrowed {
                    Self::Map {
                        function,
                        ..
                    } =>
                        Composite::Map {
                            function:
                                function.clone(),
                        },

                    Self::Filter {
                        predicate,
                        ..
                    } =>
                        Composite::Filter {
                            predicate:
                                predicate.clone(),
                        },

                    Self::Enumerate {
                        source,
                        index,
                    } =>
                        Composite::Enumerate {
                            source:
                                source.clone(),
                            index:
                                *index,
                        },

                    Self::Zip {
                        left,
                        right,
                    } =>
                        Composite::Zip {
                            left:
                                left.clone(),
                            right:
                                right.clone(),
                        },

                    Self::Take {
                        source,
                        remaining,
                    } =>
                        Composite::Take {
                            source:
                                source.clone(),
                            remaining:
                                *remaining,
                        },

                    Self::Skip {
                        source,
                        remaining,
                    } =>
                        Composite::Skip {
                            source:
                                source.clone(),
                            remaining:
                                *remaining,
                        },

                    Self::List { .. }
                    | Self::Str { .. }
                    | Self::Vector { .. }
                    | Self::Range { .. } =>
                        unreachable!(
                            "next_composite called for primitive iterator"
                        ),
                }
            };

        match composite {
            /*
            * Map / Filter remain VM-managed.
            */
            Composite::Map { .. }
            |
            Composite::Filter { .. } =>
                Err(
                    "Map/Filter iterators must be evaluated by the VM"
                        .into()
                ),

            /*
            * Enumerate
            */
            Composite::Enumerate {
                source,
                index,
            } => {
                match Self::next(
                    &source
                )? {
                    IterResult::Item(
                        value
                    ) => {
                        {
                            let mut state =
                                iterator.borrow_mut();

                            let Self::Enumerate {
                                index,
                                ..
                            } =
                                &mut *state
                            else {
                                unreachable!();
                            };

                            *index += 1;
                        }

                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(vec![
                                        Value::Int(
                                            index as i64
                                        ),
                                        value,
                                    ])
                                )
                            )
                        )
                    }

                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * Zip
            */
            Composite::Zip {
                left,
                right,
            } => {
                let lhs =
                    Self::next(
                        &left
                    )?;

                let rhs =
                    Self::next(
                        &right
                    )?;

                match (
                    lhs,
                    rhs,
                ) {
                    (
                        IterResult::Item(lhs),
                        IterResult::Item(rhs),
                    ) =>
                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(vec![
                                        lhs,
                                        rhs,
                                    ])
                                )
                            )
                        ),

                    _ =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            /*
            * Take
            */
            Composite::Take {
                source,
                remaining,
            } => {
                if remaining == 0 {
                    return Ok(
                        IterResult::End
                    );
                }

                let result =
                    Self::next(
                        &source
                    )?;

                if matches!(
                    result,
                    IterResult::Item(_)
                ) {
                    let mut state =
                        iterator.borrow_mut();

                    let Self::Take {
                        remaining,
                        ..
                    } =
                        &mut *state
                    else {
                        unreachable!();
                    };

                    *remaining -= 1;
                }

                Ok(result)
            }

            /*
            * Skip
            */
            Composite::Skip {
                source,
                remaining,
            } => {
                let mut remaining =
                    remaining;

                while remaining > 0 {
                    match Self::next(
                        &source
                    )? {
                        IterResult::Item(_) => {
                            remaining -= 1;
                        }

                        IterResult::End =>
                            return Ok(
                                IterResult::End
                            ),
                    }
                }

                {
                    let mut state =
                        iterator.borrow_mut();

                    let Self::Skip {
                        remaining,
                        ..
                    } =
                        &mut *state
                    else {
                        unreachable!();
                    };

                    *remaining =
                        0;
                }

                Self::next(
                    &source
                )
            }
        }
    }

}

