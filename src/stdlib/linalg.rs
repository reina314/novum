use crate::runtime::{ExtensionRegistry, List, Matrix, Module, ModuleRef, ReceiverKind, Value};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

struct FunctionSpec {
    name: &'static str,
    function: fn(Vec<Value>) -> Result<Value, String>,
    receiver: Option<ReceiverKind>,
}

fn function_specs() -> &'static [FunctionSpec] {
    &[
        FunctionSpec {
            name: "vector",
            function: vector,
            receiver: Some(ReceiverKind::List),
        },
        FunctionSpec {
            name: "matrix",
            function: matrix,
            receiver: Some(ReceiverKind::List),
        },
        FunctionSpec {
            name: "transpose",
            function: transpose,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "det",
            function: det,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "inverse",
            function: inverse,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "solve",
            function: solve,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "solve_lstsq",
            function: solve_lstsq,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "shape",
            function: shape,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "rows",
            function: rows,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "cols",
            function: cols,
            receiver: Some(ReceiverKind::Matrix),
        },
        FunctionSpec {
            name: "linear_regression",
            function: linear_regression,
            receiver: Some(ReceiverKind::Matrix),
        },
    ]
}

pub fn register_extensions(registry: &mut ExtensionRegistry) {
    for spec in function_specs() {
        let Some(receiver) = spec.receiver else {
            continue;
        };

        registry.register(receiver, spec.name, Value::Builtin(spec.function));
    }
}

pub fn module() -> ModuleRef {
    let mut module = Module::new("linalg");

    for spec in function_specs() {
        module.set_exported(spec.name, Value::Builtin(spec.function));
    }

    Rc::new(RefCell::new(module))
}

fn value_to_f64(value: &Value) -> Result<f64, String> {
    match value {
        Value::Int(x) => Ok(*x as f64),

        Value::Float(x) => Ok(*x),

        other => Err(format!("expected numeric value, got {}", other.type_name())),
    }
}

fn value_to_matrix_rows(value: &Value) -> Result<Vec<Vec<f64>>, String> {
    let rows = match value {
        Value::List(list) => list.iter_cloned(),

        other => {
            return Err(format!("matrix() expects List, got {}", other.type_name()));
        },
    };

    let mut result = Vec::with_capacity(rows.len());

    for row in rows {
        let values = match row {
            Value::List(row) => row.iter_cloned(),

            other => {
                return Err(format!(
                    "matrix rows must be List, got {}",
                    other.type_name()
                ));
            },
        };

        let mut converted = Vec::with_capacity(values.len());

        for value in values {
            converted.push(value_to_f64(&value)?);
        }

        result.push(converted);
    }

    Ok(result)
}

pub fn vector(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vector() expects exactly 1 argument".into());
    }

    let values = match &args[0] {
        Value::List(list) => list.iter_cloned(),

        other => {
            return Err(format!("vector() expects List, got {}", other.type_name()));
        },
    };

    let mut data = Vec::with_capacity(values.len());

    for value in values {
        data.push(value_to_f64(&value)?);
    }

    let vector = crate::runtime::Vector::new(data);

    Ok(Value::Vector(Rc::new(RefCell::new(vector))))
}

pub fn matrix(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("matrix() expects exactly 1 argument".into());
    }

    let rows = value_to_matrix_rows(&args[0])?;

    let matrix = Matrix::from_rows(rows)?;

    Ok(Value::Matrix(Rc::new(RefCell::new(matrix))))
}

pub fn transpose(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("transpose() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "transpose() expects Matrix, got {}",
                other.type_name()
            ));
        },
    };

    let result = matrix.transpose();

    Ok(Value::Matrix(Rc::new(RefCell::new(result))))
}

pub fn det(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("det() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!("det() expects Matrix, got {}", other.type_name()));
        },
    };

    Ok(Value::Float(matrix.determinant()?))
}

pub fn inverse(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("inverse() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "inverse() expects Matrix, got {}",
                other.type_name()
            ));
        },
    };

    let result = matrix.inverse()?;

    Ok(Value::Matrix(Rc::new(RefCell::new(result))))
}

pub fn solve(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("solve() expects A and b".into());
    }

    let a = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "solve() A must be Matrix, got {}",
                other.type_name()
            ));
        },
    };

    match &args[1] {
        Value::Matrix(rhs) => {
            let rhs = rhs.borrow();

            let result = a.solve(&rhs)?;

            Ok(Value::Matrix(Rc::new(RefCell::new(result))))
        },

        Value::Vector(rhs) => {
            let rhs = rhs.borrow();

            let rhs_matrix = rhs.to_column_matrix();

            let result = a.solve(&rhs_matrix)?;

            let vector = crate::runtime::Vector::from_matrix_column(&result)?;

            Ok(Value::Vector(Rc::new(RefCell::new(vector))))
        },

        other => Err(format!(
            "solve() b must be Matrix or Vector, got {}",
            other.type_name()
        )),
    }
}

pub fn solve_lstsq(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("solve_lstsq() expects A and b".into());
    }

    let a = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "solve_lstsq() A must be Matrix, got {}",
                other.type_name()
            ));
        },
    };

    match &args[1] {
        Value::Matrix(rhs) => {
            let rhs = rhs.borrow();

            let result = a.solve_lstsq(&rhs)?;

            Ok(Value::Matrix(Rc::new(RefCell::new(result))))
        },

        Value::Vector(rhs) => {
            let rhs = rhs.borrow();

            let rhs_matrix = rhs.to_column_matrix();

            let result = a.solve_lstsq(&rhs_matrix)?;

            let vector = crate::runtime::Vector::from_matrix_column(&result)?;

            Ok(Value::Vector(Rc::new(RefCell::new(vector))))
        },

        other => Err(format!(
            "solve_lstsq() b must be Matrix or Vector, got {}",
            other.type_name()
        )),
    }
}

pub fn shape(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("shape() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!("shape() expects Matrix, got {}", other.type_name()));
        },
    };

    let (rows, cols) = matrix.shape();

    Ok(Value::List(List::new(vec![
        Value::Int(rows as i64),
        Value::Int(cols as i64),
    ])))
}

pub fn rows(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("rows() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!("rows() expects Matrix, got {}", other.type_name()));
        },
    };

    Ok(Value::Int(matrix.rows() as i64))
}

pub fn cols(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cols() expects exactly 1 argument".into());
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!("cols() expects Matrix, got {}", other.type_name()));
        },
    };

    Ok(Value::Int(matrix.cols() as i64))
}

pub fn linear_regression(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("linear_regression() expects X and y".into());
    }

    let x = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "linear_regression() X must be Matrix, got {}",
                other.type_name()
            ));
        },
    };

    let y = match &args[1] {
        Value::Matrix(matrix) => matrix.borrow(),

        Value::Vector(vector) => {
            let vector = vector.borrow();

            let y = vector.to_column_matrix();

            let x_rows = x.rows();

            if x_rows != y.rows() {
                return Err(format!(
                    "linear_regression() X and y must have the same number of rows: {} vs {}",
                    x_rows,
                    y.rows(),
                ));
            }

            let coefficients = x.solve_lstsq(&y)?;

            let predicted = x.matmul(&coefficients)?;

            let mut y_mean = 0.0;

            for row in 0..y.rows() {
                y_mean += y
                    .get(row, 0)
                    .ok_or_else(|| format!("failed to access y[{}, 0]", row))?;
            }

            y_mean /= y.rows() as f64;

            let mut ss_res = 0.0;

            let mut ss_tot = 0.0;

            for row in 0..y.rows() {
                let actual = y
                    .get(row, 0)
                    .ok_or_else(|| format!("failed to access y[{}, 0]", row))?;

                let fitted = predicted
                    .get(row, 0)
                    .ok_or_else(|| format!("failed to access predicted[{}, 0]", row))?;

                let residual = actual - fitted;

                let deviation = actual - y_mean;

                ss_res += residual * residual;

                ss_tot += deviation * deviation;
            }

            let r_squared = if ss_tot == 0.0 {
                if ss_res == 0.0 {
                    1.0
                } else {
                    f64::NAN
                }
            } else {
                1.0 - ss_res / ss_tot
            };

            let mut result = HashMap::new();

            result.insert(
                "coefficients".to_string(),
                Value::Matrix(Rc::new(RefCell::new(coefficients))),
            );

            result.insert(
                "fitted".to_string(),
                Value::Matrix(Rc::new(RefCell::new(predicted))),
            );

            result.insert("r_squared".to_string(), Value::Float(r_squared));

            result.insert("residual_sum_of_squares".to_string(), Value::Float(ss_res));

            return Ok(Value::Dict(Rc::new(RefCell::new(result))));
        },

        other => {
            return Err(format!(
                "linear_regression() y must be Matrix or Vector, got {}",
                other.type_name()
            ));
        },
    };

    if x.rows() == 0 {
        return Err("linear_regression() requires at least one observation".into());
    }

    if y.cols() != 1 {
        return Err("linear_regression() y must be a column Matrix".into());
    }

    if x.rows() != y.rows() {
        return Err(format!(
            "linear_regression() X and y must have the same number of rows: {} vs {}",
            x.rows(),
            y.rows(),
        ));
    }

    if x.cols() == 0 {
        return Err("linear_regression() X must have at least one feature".into());
    }

    let coefficients = x.solve_lstsq(&y)?;

    let predicted = x.matmul(&coefficients)?;

    let mut y_mean = 0.0;

    for row in 0..y.rows() {
        y_mean += y
            .get(row, 0)
            .ok_or_else(|| format!("failed to access y[{}, 0]", row))?;
    }

    y_mean /= y.rows() as f64;

    let mut ss_res = 0.0;

    let mut ss_tot = 0.0;

    for row in 0..y.rows() {
        let actual = y
            .get(row, 0)
            .ok_or_else(|| format!("failed to access y[{}, 0]", row))?;

        let fitted = predicted
            .get(row, 0)
            .ok_or_else(|| format!("failed to access predicted[{}, 0]", row))?;

        let residual = actual - fitted;

        let deviation = actual - y_mean;

        ss_res += residual * residual;

        ss_tot += deviation * deviation;
    }

    let r_squared = if ss_tot == 0.0 {
        if ss_res == 0.0 {
            1.0
        } else {
            f64::NAN
        }
    } else {
        1.0 - ss_res / ss_tot
    };

    let mut result = HashMap::new();

    result.insert(
        "coefficients".to_string(),
        Value::Matrix(Rc::new(RefCell::new(coefficients))),
    );

    result.insert(
        "fitted".to_string(),
        Value::Matrix(Rc::new(RefCell::new(predicted))),
    );

    result.insert("r_squared".to_string(), Value::Float(r_squared));

    result.insert("residual_sum_of_squares".to_string(), Value::Float(ss_res));

    Ok(Value::Dict(Rc::new(RefCell::new(result))))
}
