use crate::{
    runtime::{
        Value,
        Vector,
        Series,
        SeriesRef,
    },
    syntax::BinOp,
};

use std::{
    cell::RefCell,
    rc::Rc,
};

fn apply_scalar_binop(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    use BinOp::*;

    match op {
        Add => add(lhs, rhs),
        Sub => sub(lhs, rhs),
        Mul => mul(lhs, rhs),
        Div => divide(lhs, rhs),

        Pow => power(lhs, rhs),
        Mod => modulo(lhs, rhs),
        MatMul => matmul(lhs, rhs),

        Eq => Ok(
            Value::Bool(
                Value::eq_values(
                    &lhs,
                    &rhs,
                )?
            )),

        Neq => Ok(
            Value::Bool(
                !Value::eq_values(
                    &lhs,
                    &rhs,
                )?
            )),

        Lt | Leq | Gt | Geq =>
            compare(op, lhs, rhs),

        And | Or =>
            bool_binary(op, lhs, rhs),
    }
}

fn apply_series_binop(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Series(lhs),
            Value::Series(rhs),
        ) => {
            if lhs.len() != rhs.len() {
                return Err(format!(
                    "Series length mismatch: {} vs {}",
                    lhs.len(),
                    rhs.len()
                ));
            }

            let mut values =
                Vec::with_capacity(
                    lhs.len()
                );

            for i in 0..lhs.len() {
                let left =
                    lhs.get(i).unwrap();

                let right =
                    rhs.get(i).unwrap();

                values.push(
                    apply_series_element(
                        op,
                        left,
                        right,
                    )?
                );
            }

            Ok(
                Value::Series(
                    Rc::new(
                        Series::new(
                            lhs.name(),
                            values,
                        )
                    )
                )
            )
        }

        (
            Value::Series(series),
            scalar,
        ) => {
            let values =
                series
                    .data()
                    .iter()
                    .cloned()
                    .map(|value| {
                        apply_series_element(
                            op,
                            value,
                            scalar.clone(),
                        )
                    })
                    .collect::<Result<
                        Vec<_>,
                        _
                    >>()?;

            Ok(
                Value::Series(
                    Rc::new(
                        Series::new(
                            series.name(),
                            values,
                        )
                    )
                )
            )
        }

        (
            scalar,
            Value::Series(series),
        ) => {
            let values =
                series
                    .data()
                    .iter()
                    .cloned()
                    .map(|value| {
                        apply_series_element(
                            op,
                            scalar.clone(),
                            value,
                        )
                    })
                    .collect::<Result<
                        Vec<_>,
                        _
                    >>()?;

            Ok(
                Value::Series(
                    Rc::new(
                        Series::new(
                            series.name(),
                            values,
                        )
                    )
                )
            )
        }

        _ => unreachable!(),
    }
}

fn apply_series_element(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match op {
        BinOp::Eq => {
            return Ok(
                Value::Bool(
                    Value::eq_values(
                        &lhs,
                        &rhs,
                    )?
                )
            );
        }

        BinOp::Neq => {
            return Ok(
                Value::Bool(
                    !Value::eq_values(
                        &lhs,
                        &rhs,
                    )?
                )
            );
        }

        _ => {}
    }

    if matches!(lhs, Value::Null)
        || matches!(rhs, Value::Null)
    {
        return Ok(Value::Null);
    }

    apply_scalar_binop(
        op,
        lhs,
        rhs,
    )
}

pub fn apply_series_boolean_op(
    lhs: SeriesRef,
    rhs: SeriesRef,
    is_or: bool,
) -> Result<Series, String> {
    if lhs.len() != rhs.len() {
        return Err(format!(
            "Series length mismatch: {} vs {}",
            lhs.len(),
            rhs.len()
        ));
    }

    let mut values =
        Vec::with_capacity(
            lhs.len()
        );

    for i in 0..lhs.len() {
        let left =
            lhs.get(i)
                .expect("Series index in bounds");

        let right =
            rhs.get(i)
                .expect("Series index in bounds");

        let value =
            match (left, right) {
                // -------------------------------------------------
                // Bool × Bool
                // -------------------------------------------------

                (
                    Value::Bool(a),
                    Value::Bool(b),
                ) => {
                    Value::Bool(
                        if is_or {
                            a || b
                        } else {
                            a && b
                        }
                    )
                }

                // -------------------------------------------------
                // Missing value
                // -------------------------------------------------

                (
                    Value::Null,
                    _
                )
                |
                (
                    _,
                    Value::Null,
                ) => {
                    Value::Null
                }

                // -------------------------------------------------
                // Type error
                // -------------------------------------------------

                (a, b) => {
                    return Err(format!(
                        "logical operation requires Bool values, got {} and {}",
                        a.type_name(),
                        b.type_name()
                    ));
                }
            };

        values.push(value);
    }

    Ok(
        Series::new(
            lhs.name(),
            values,
        )
    )
}

pub fn apply_binop(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (&lhs, &rhs) {
        (Value::Series(_), _)
        | (_, Value::Series(_)) => {
            apply_series_binop(
                op,
                lhs,
                rhs,
            )
        }

        _ => {
            apply_scalar_binop(
                op,
                lhs,
                rhs,
            )
        }
    }
}

fn add(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        // string concatenation
        (
            Value::Str(a),
            Value::Str(b),
        ) => {
            Ok(Value::Str(
                Rc::new(
                    format!("{}{}", a, b)
                )
            ))
        }

        (
            Value::Str(a),
            b,
        ) => {
            Ok(Value::Str(
                Rc::new(
                    format!("{}{}", a, b)
                )
            ))
        }

        (
            a,
            Value::Str(b),
        ) => {
            Ok(Value::Str(
                Rc::new(
                    format!("{}{}", a, b)
                )
            ))
        }

        // integers
        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            a.checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| {
                    "integer overflow".into()
                })
        }

        // floating point
        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(a + b))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(
                a as f64 + b
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            Ok(Value::Float(
                a + b as f64
            ))
        }

        (
            Value::Vector(a),
            Value::Vector(b),
        ) => {
            let result =
                a.borrow()
                    .add(
                        &b.borrow()
                    )?;

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        (
            Value::Matrix(a),
            Value::Matrix(b),
        ) => {
            let a = a.borrow();
            let b = b.borrow();

            let result = a.add(&b)?;

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        (a, b) => {
            Err(format!(
                "addition not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn sub(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            a.checked_sub(b)
                .map(Value::Int)
                .ok_or_else(|| {
                    "integer overflow".into()
                })
        }

        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(a - b))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(
                a as f64 - b
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            Ok(Value::Float(
                a - b as f64
            ))
        }

        (
            Value::Vector(a),
            Value::Vector(b),
        ) => {
            let result =
                a.borrow()
                    .sub(
                        &b.borrow()
                    )?;

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        (
            Value::Matrix(a),
            Value::Matrix(b),
        ) => {
            let a = a.borrow();
            let b = b.borrow();

            let result = a.sub(&b)?;

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        (a, b) => {
            Err(format!(
                "subtraction not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn mul(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            a.checked_mul(b)
                .map(Value::Int)
                .ok_or_else(|| {
                    "integer overflow".into()
                })
        }

        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(a * b))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(
                a as f64 * b
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            Ok(Value::Float(
                a * b as f64
            ))
        }

        (
            Value::Vector(vector),
            Value::Int(scalar),
        ) => {
            let result =
                vector.borrow()
                    .scale(
                        scalar as f64
                    );

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        (
            Value::Vector(vector),
            Value::Float(scalar),
        ) => {
            let result =
                vector.borrow()
                    .scale(scalar);

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        (
            Value::Int(scalar),
            Value::Vector(vector),
        ) => {
            let result =
                vector.borrow()
                    .scale(
                        scalar as f64
                    );

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        (
            Value::Float(scalar),
            Value::Vector(vector),
        ) => {
            let result =
                vector.borrow()
                    .scale(scalar);

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            result
                        )
                    )
                )
            )
        }

        // Matrix * Matrix = element-wise
        (
            Value::Matrix(a),
            Value::Matrix(b),
        ) => {
            let a = a.borrow();
            let b = b.borrow();

            let result =
                a.elementwise_mul(&b)?;

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        // scalar * Matrix
        (
            Value::Int(a),
            Value::Matrix(b),
        ) => {
            let result =
                b.borrow()
                    .scalar_mul(a as f64);

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        (
            Value::Float(a),
            Value::Matrix(b),
        ) => {
            let result =
                b.borrow()
                    .scalar_mul(a);

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        // Matrix * scalar
        (
            Value::Matrix(a),
            Value::Int(b),
        ) => {
            let result =
                a.borrow()
                    .scalar_mul(b as f64);

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                ))
            )
        }

        (
            Value::Matrix(a),
            Value::Float(b),
        ) => {
            let result =
                a.borrow()
                    .scalar_mul(b);

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                ))
            )
        }

        (a, b) => {
            Err(format!(
                "multiplication not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn divide(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            if b == 0 {
                return Err(
                    "division by zero".into()
                );
            }

            if a % b == 0 {
                a.checked_div(b)
                    .map(Value::Int)
                    .ok_or_else(|| {
                        "integer overflow".into()
                    })
            } else {
                Ok(Value::Float(
                    a as f64 / b as f64
                ))
            }
        }

        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            if b == 0.0 {
                return Err(
                    "division by zero".into()
                );
            }

            Ok(Value::Float(a / b))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            if b == 0.0 {
                return Err(
                    "division by zero".into()
                );
            }

            Ok(Value::Float(
                a as f64 / b
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            if b == 0 {
                return Err(
                    "division by zero".into()
                );
            }

            Ok(Value::Float(
                a / b as f64
            ))
        }

        (a, b) => {
            Err(format!(
                "division not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn power(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) if b >= 0
            && b <= u32::MAX as i64 =>
        {
            a.checked_pow(b as u32)
                .map(Value::Int)
                .ok_or_else(|| {
                    "integer overflow in power".into()
                })
        }

        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            Ok(Value::Float(
                (a as f64).powf(b as f64)
            ))
        }

        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(a.powf(b)))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            Ok(Value::Float(
                (a as f64).powf(b)
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            Ok(Value::Float(
                a.powf(b as f64)
            ))
        }

        (a, b) => {
            Err(format!(
                "power not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn modulo(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) => {
            if b == 0 {
                return Err(
                    "modulo by zero".into()
                );
            }

            Ok(Value::Int(a % b))
        }

        (
            Value::Float(a),
            Value::Float(b),
        ) => {
            if b == 0.0 {
                return Err(
                    "modulo by zero".into()
                );
            }

            Ok(Value::Float(a % b))
        }

        (
            Value::Int(a),
            Value::Float(b),
        ) => {
            if b == 0.0 {
                return Err(
                    "modulo by zero".into()
                );
            }

            Ok(Value::Float(
                a as f64 % b
            ))
        }

        (
            Value::Float(a),
            Value::Int(b),
        ) => {
            if b == 0 {
                return Err(
                    "modulo by zero".into()
                );
            }

            Ok(Value::Float(
                a % b as f64
            ))
        }

        (a, b) => {
            Err(format!(
                "modulo not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn matmul(
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Vector(a),
            Value::Vector(b),
        ) => {
            let result =
                a.borrow()
                    .dot(
                        &b.borrow()
                    )?;

            Ok(
                Value::Float(result)
            )
        }
        
        (
            Value::Matrix(a),
            Value::Matrix(b),
        ) => {
            let a = a.borrow();
            let b = b.borrow();

            let result =
                a.matmul(&b)?;

            Ok(Value::Matrix(
                Rc::new(
                    RefCell::new(result)
                )
            ))
        }

        (
            Value::Vector(vector),
            Value::Matrix(matrix),
        ) => {
            let vector =
                vector.borrow();

            let matrix =
                matrix.borrow();

            if vector.len()
                != matrix.rows()
            {
                return Err(format!(
                    "vector-matrix multiplication dimension mismatch: vector length {}, matrix shape ({}, {})",
                    vector.len(),
                    matrix.rows(),
                    matrix.cols(),
                ));
            }

            let mut result =
                vec![
                    0.0;
                    matrix.cols()
                ];

            for c in 0..matrix.cols() {
                let mut sum =
                    0.0;

                for r in 0..matrix.rows() {
                    let v =
                        vector
                            .get(r)
                            .expect(
                                "vector index out of bounds"
                            );

                    let m =
                        matrix
                            .get(r, c)
                            .expect(
                                "matrix index out of bounds"
                            );

                    sum += v * m;
                }

                result[c] = sum;
            }

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            Vector::new(result)
                        )
                    )
                )
            )
        }

        (
            Value::Matrix(matrix),
            Value::Vector(vector),
        ) => {
            let matrix =
                matrix.borrow();

            let vector =
                vector.borrow();

            if matrix.cols()
                != vector.len()
            {
                return Err(format!(
                    "matrix-vector multiplication dimension mismatch: matrix shape ({}, {}), vector length {}",
                    matrix.rows(),
                    matrix.cols(),
                    vector.len(),
                ));
            }

            let mut result =
                vec![
                    0.0;
                    matrix.rows()
                ];

            for r in 0..matrix.rows() {
                let mut sum =
                    0.0;

                for c in 0..matrix.cols() {
                    let m =
                        matrix
                            .get(r, c)
                            .expect(
                                "matrix index out of bounds"
                            );

                    let v =
                        vector
                            .get(c)
                            .expect(
                                "vector index out of bounds"
                            );

                    sum += m * v;
                }

                result[r] = sum;
            }

            Ok(
                Value::Vector(
                    Rc::new(
                        RefCell::new(
                            Vector::new(result)
                        )
                    )
                )
            )
        }

        (a, b) => {
            Err(format!(
                "'@' expects Matrix or Vector, got {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn bool_binary(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match (lhs, rhs) {
        (
            Value::Bool(a),
            Value::Bool(b),
        ) => {
            let result = match op {
                BinOp::And => a && b,
                BinOp::Or => a || b,

                _ => unreachable!(
                    "bool_binary called with non-logical operator"
                ),
            };

            Ok(Value::Bool(result))
        }

        (a, b) => {
            Err(format!(
                "logical operation not defined between {} and {}",
                a.type_name(),
                b.type_name()
            ))
        }
    }
}

fn compare(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    fn ord(
        a: f64,
        b: f64,
        op: BinOp,
    ) -> bool {
        match op {
            BinOp::Lt => a < b,
            BinOp::Leq => a <= b,
            BinOp::Gt => a > b,
            BinOp::Geq => a >= b,

            _ => unreachable!(
                "ord called with non-comparison operator"
            ),
        }
    }

    let result = match (&lhs, &rhs) {
        (
            Value::Int(a),
            Value::Int(b),
        ) => ord(
            *a as f64,
            *b as f64,
            op,
        ),

        (
            Value::Float(a),
            Value::Float(b),
        ) => ord(*a, *b, op),

        (
            Value::Int(a),
            Value::Float(b),
        ) => ord(
            *a as f64,
            *b,
            op,
        ),

        (
            Value::Float(a),
            Value::Int(b),
        ) => ord(
            *a,
            *b as f64,
            op,
        ),

        (
            Value::Str(a),
            Value::Str(b),
        ) => {
            match op {
                BinOp::Lt =>
                    a < b,

                BinOp::Leq =>
                    a <= b,

                BinOp::Gt =>
                    a > b,

                BinOp::Geq =>
                    a >= b,

                _ => unreachable!(),
            }
        }

        _ => {
            return Err(format!(
                "comparison not defined between {} and {}",
                lhs.type_name(),
                rhs.type_name()
            ));
        }
    };

    Ok(Value::Bool(result))
}