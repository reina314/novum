use crate::{
    runtime::Value,
    syntax::BinOp,
};

use std::{
    cell::RefCell,
    rc::Rc,
};


pub fn apply_binop(
    op: BinOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, String> {
    match op {
        BinOp::Add =>
            add(lhs, rhs),

        BinOp::Sub =>
            sub(lhs, rhs),

        BinOp::Mul =>
            mul(lhs, rhs),

        BinOp::Div =>
            divide(lhs, rhs),

        BinOp::Pow =>
            power(lhs, rhs),

        BinOp::Mod =>
            modulo(lhs, rhs),

        BinOp::MatMul =>
            matmul(lhs, rhs),

        BinOp::Eq =>
            Ok(Value::Bool(
                Value::eq_values(
                    &lhs,
                    &rhs,
                )?
            )),

        BinOp::Neq =>
            Ok(Value::Bool(
                !Value::eq_values(
                    &lhs,
                    &rhs,
                )?
            )),

        BinOp::Lt
        | BinOp::Leq
        | BinOp::Gt
        | BinOp::Geq =>
            compare(op, lhs, rhs),

        // NOTE:
        // And / Or should normally be short-circuited
        // by eval.rs BEFORE evaluating rhs.
        BinOp::And
        | BinOp::Or =>
            bool_binary(op, lhs, rhs),
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

        // Matrix
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

        (a, b) => {
            Err(format!(
                "'@' expects Matrix @ Matrix, got {} and {}",
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