use super::{
    List,
    Value,
    StrRef,
    VectorRef,
    SeriesRef,
    DataFrameRef,
    ClosureRef,
};
use std::{
    cell::RefCell, rc::Rc,
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

    Series {
        data: SeriesRef,
        index: usize,
    },

    DataFrame {
        data: DataFrameRef,
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

            Value::Str(data) =>
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Str {
                                data,
                                byte_index: 0,
                            }
                        )
                    )
                ),

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

            Value::Vector(data) =>
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Vector {
                                data,
                                index: 0,
                            }
                        )
                    )
                ),

            Value::Series(data) =>
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::Series {
                                data,
                                index: 0,
                            }
                        )
                    )
                ),

            Value::DataFrame(data) =>
                Ok(
                    Rc::new(
                        RefCell::new(
                            IteratorObj::DataFrame {
                                data,
                                index: 0,
                            }
                        )
                    )
                ),

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
        /*
        * Try primitive path first.
        *
        * This borrow is held only for the duration of the
        * primitive operation. Composite iterators are extracted
        * into owned handles and the borrow is released before
        * recursive evaluation.
        */
        {
            let mut state =
                iterator.borrow_mut();

            match &mut *state {
                Self::List {
                    data,
                    index,
                } => {
                    let value =
                        data.get(*index);

                    match value {
                        Some(value) => {
                            *index += 1;

                            return Ok(
                                IterResult::Item(
                                    value
                                )
                            );
                        }

                        None => {
                            return Ok(
                                IterResult::End
                            );
                        }
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

                    return Ok(
                        IterResult::Item(
                            Value::Str(
                                Rc::new(
                                    ch.to_string()
                                )
                            )
                        )
                    );
                }

                Self::Series {
                    data,
                    index,
                } => {
                    let value =
                        data.get(*index);

                    match value {
                        Some(value) => {
                            *index += 1;

                            return Ok(
                                IterResult::Item(
                                    value
                                )
                            );
                        }

                        None =>
                            return Ok(
                                IterResult::End
                            ),
                    }
                }

                Self::DataFrame {
                    data,
                    index,
                } => {
                    match data.row(*index) {
                        Some(row) => {
                            *index += 1;

                            return Ok(
                                IterResult::Item(
                                    Value::Dict(row)
                                )
                            );
                        }

                        None =>
                            return Ok(
                                IterResult::End
                            ),
                    }
                }

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

                            return Ok(
                                IterResult::Item(
                                    Value::Float(
                                        value
                                    )
                                )
                            );
                        }

                        None => {
                            return Ok(
                                IterResult::End
                            );
                        }
                    }
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

                    return Ok(
                        IterResult::Item(
                            Value::Int(
                                value
                            )
                        )
                    );
                }

                _ => {}
            }
        }

        /*
        * We only reach here for composite iterators.
        *
        * Their handles/state are copied out of the RefCell,
        * then all recursive calls happen after the borrow is gone.
        */
        Self::next_composite(
            iterator
        )
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

        let composite = {
            let state =
                iterator.borrow();

            match &*state {
                Self::Map {
                    function,
                    ..
                } => {
                    Composite::Map {
                        function:
                            function.clone(),
                    }
                }

                Self::Filter {
                    predicate,
                    ..
                } => {
                    Composite::Filter {
                        predicate:
                            predicate.clone(),
                    }
                }

                Self::Enumerate {
                    source,
                    index,
                } => {
                    Composite::Enumerate {
                        source:
                            source.clone(),
                        index:
                            *index,
                    }
                }

                Self::Zip {
                    left,
                    right,
                } => {
                    Composite::Zip {
                        left:
                            left.clone(),
                        right:
                            right.clone(),
                    }
                }

                Self::Take {
                    source,
                    remaining,
                } => {
                    Composite::Take {
                        source:
                            source.clone(),
                        remaining:
                            *remaining,
                    }
                }

                Self::Skip {
                    source,
                    remaining,
                } => {
                    Composite::Skip {
                        source:
                            source.clone(),
                        remaining:
                            *remaining,
                    }
                }

                Self::List { .. }
                | Self::Str { .. }
                | Self::Series { .. }
                | Self::DataFrame { .. }
                | Self::Vector { .. }
                | Self::Range { .. } => {
                    unreachable!(
                        "next_composite called for primitive iterator"
                    )
                }
            }
        };

        match composite {
            Composite::Map { .. }
            | Composite::Filter { .. } => {
                Err(
                    "Map/Filter iterators must be evaluated by the VM"
                        .into()
                )
            }

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
                        let current_index = index;

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

                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(vec![
                                        Value::Int(
                                            current_index as i64
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
                    ) => {
                        Ok(
                            IterResult::Item(
                                Value::Tuple(
                                    Rc::new(vec![
                                        lhs,
                                        rhs,
                                    ])
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

                        IterResult::End => {
                            return Ok(
                                IterResult::End
                            );
                        }
                    }
                }

                {
                    let mut state =
                        iterator.borrow_mut();

                    let Self::Skip {
                        remaining:
                            state_remaining,
                        ..
                    } =
                        &mut *state
                    else {
                        unreachable!();
                    };

                    *state_remaining =
                        0;
                }

                Self::next(
                    &source
                )
            }
        }
    }

}

