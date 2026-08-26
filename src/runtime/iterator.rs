use super::{
    List,
    ListRef,
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
        let variant =
            iterator.borrow().clone();

        match variant {
            Self::List {
                data,
                index,
            } => {
                let value =
                    data.get(index);

                match value {
                    Some(value) => {
                        if let Self::List {
                            index,
                            ..
                        } = &mut *iterator.borrow_mut()
                        {
                            *index += 1;
                        }

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
                    &data[byte_index..];

                let Some(ch) =
                    slice.chars().next()
                else {
                    return Ok(
                        IterResult::End
                    );
                };

                if let Self::Str {
                    byte_index,
                    ..
                } = &mut *iterator.borrow_mut()
                {
                    *byte_index +=
                        ch.len_utf8();
                }

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
                if current >= end {
                    return Ok(
                        IterResult::End
                    );
                }

                if let Self::Range {
                    current,
                    ..
                } = &mut *iterator.borrow_mut()
                {
                    *current += 1;
                }

                Ok(
                    IterResult::Item(
                        Value::Int(current)
                    )
                )
            }

            Self::Vector {
                data,
                index,
            } => {
                let value =
                    data.borrow()
                        .get(index)
                        .map(|value| {
                            Value::Float(value)
                        });

                match value {
                    Some(value) => {
                        if let Self::Vector {
                            index,
                            ..
                        } = &mut *iterator.borrow_mut()
                        {
                            *index += 1;
                        }

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

            Self::Map { .. } |
            Self::Filter { .. } => {
                Err(
                    "Map/Filter iterators must be evaluated by the VM"
                        .into()
                )
            }

            Self::Enumerate {
                source,
                index: _,
            } => {
                match Self::next(&source)? {
                    IterResult::Item(value) => {
                        if let Self::Enumerate {
                            index,
                            ..
                        } = &mut *iterator.borrow_mut()
                        {
                            let current =
                                *index;

                            *index += 1;

                            Ok(
                                IterResult::Item(
                                    Value::Tuple(
                                        Rc::new(vec![
                                            Value::Int(
                                                current as i64
                                            ),
                                            value,
                                        ])
                                    )
                                )
                            )
                        } else {
                            unreachable!()
                        }
                    }

                    IterResult::End =>
                        Ok(
                            IterResult::End
                        ),
                }
            }

            Self::Zip {
                left,
                right,
            } => {
                let lhs =
                    Self::next(&left)?;

                let rhs =
                    Self::next(&right)?;

                match (lhs, rhs) {
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

            Self::Take {
                source,
                remaining,
            } => {
                if remaining == 0 {
                    return Ok(
                        IterResult::End
                    );
                }

                let result =
                    Self::next(&source)?;

                if matches!(
                    result,
                    IterResult::Item(_)
                ) {
                    if let Self::Take {
                        remaining,
                        ..
                    } = &mut *iterator.borrow_mut()
                    {
                        *remaining -= 1;
                    }
                }

                Ok(result)
            }

            Self::Skip {
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

                if let Self::Skip {
                    remaining: slot,
                    ..
                } = &mut *iterator.borrow_mut()
                {
                    *slot = 0;
                }

                Self::next(&source)
            }
        }
    }

}

