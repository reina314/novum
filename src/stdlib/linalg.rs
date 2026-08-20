use crate::runtime::{
    Matrix,
    Object,
    Value,
};

use std::{
    cell::RefCell,
    rc::Rc,
};

fn value_to_f64(
    value: &Value,
) -> Result<f64, String> {
    match value {
        Value::Int(x) => Ok(*x as f64),
        Value::Float(x) => Ok(*x),

        other => Err(format!(
            "expected numeric value, got {}",
            other.type_name()
        )),
    }
}

fn value_to_matrix_rows(
    value: &Value,
) -> Result<Vec<Vec<f64>>, String> {
    let list = match value {
        Value::List(list) => list.borrow(),

        other => {
            return Err(format!(
                "matrix() expects List, got {}",
                other.type_name()
            ));
        }
    };

    let mut rows = Vec::new();

    for row in list.iter() {
        let row_list = match row {
            Value::List(row) => row.borrow(),

            other => {
                return Err(format!(
                    "matrix rows must be List, got {}",
                    other.type_name()
                ));
            }
        };

        let mut values = Vec::new();

        for value in row_list.iter() {
            values.push(value_to_f64(value)?);
        }

        rows.push(values);
    }

    Ok(rows)
}

pub fn matrix(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "matrix() expects exactly 1 argument"
                .into()
        );
    }

    let rows =
        value_to_matrix_rows(&args[0])?;

    let matrix =
        Matrix::from_rows(rows)?;

    Ok(Value::Matrix(
        Rc::new(RefCell::new(matrix))
    ))
}

pub fn transpose(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "transpose() expects exactly 1 argument"
                .into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "transpose() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    let result = matrix.transpose();

    Ok(Value::Matrix(
        Rc::new(RefCell::new(result))
    ))
}

pub fn det(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "det() expects exactly 1 argument"
                .into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) =>
            matrix.borrow(),

        other => {
            return Err(format!(
                "det() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    Ok(Value::Float(
        matrix.determinant()?
    ))
}

pub fn inverse(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "inverse() expects exactly 1 argument"
                .into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) =>
            matrix.borrow(),

        other => {
            return Err(format!(
                "inverse() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    let result =
        matrix.inverse()?;

    Ok(Value::Matrix(
        Rc::new(RefCell::new(result))
    ))
}

pub fn shape(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "shape() expects exactly 1 argument"
                .into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) =>
            matrix.borrow(),

        other => {
            return Err(format!(
                "shape() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    let (rows, cols) =
        matrix.shape();

    Ok(Value::List(
        Rc::new(RefCell::new(vec![
            Value::Int(rows as i64),
            Value::Int(cols as i64),
        ]))
    ))
}

pub fn rows(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "rows() expects exactly 1 argument".into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "rows() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    Ok(Value::Int(
        matrix.rows() as i64
    ))
}

pub fn cols(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(
            "cols() expects exactly 1 argument".into()
        );
    }

    let matrix = match &args[0] {
        Value::Matrix(matrix) => matrix.borrow(),

        other => {
            return Err(format!(
                "cols() expects Matrix, got {}",
                other.type_name()
            ));
        }
    };

    Ok(Value::Int(
        matrix.cols() as i64
    ))
}

pub fn linear_regression(
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(
            "linear_regression() expects X and y".into()
        );
    }

    let x = match &args[0] {
        Value::Matrix(matrix) =>
            matrix.borrow(),

        other => {
            return Err(format!(
                "linear_regression() X must be Matrix, got {}",
                other.type_name()
            ));
        }
    };

    let y = match &args[1] {
        Value::Matrix(matrix) =>
            matrix.borrow(),

        other => {
            return Err(format!(
                "linear_regression() y must be Matrix, got {}",
                other.type_name()
            ));
        }
    };

    if y.cols() != 1 {
        return Err(
            "linear_regression() y must be a column Matrix"
                .into()
        );
    }

    if x.rows() != y.rows() {
        return Err(
            "linear_regression() X and y must have the same number of rows"
                .into()
        );
    }

    let xt =
        x.transpose();

    let xtx =
        xt.matmul(&x)?;

    let xtx_inv =
        xtx.inverse()?;

    let xty =
        xt.matmul(&y)?;

    let coefficients =
        xtx_inv.matmul(&xty)?;

    let predicted =
        x.matmul(&coefficients)?;

    let mut y_mean = 0.0;

    for r in 0..y.rows() {
        y_mean +=
            y.get(r, 0).unwrap();
    }

    y_mean /=
        y.rows() as f64;

    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;

    for r in 0..y.rows() {
        let actual =
            y.get(r, 0).unwrap();

        let fitted =
            predicted.get(r, 0).unwrap();

        ss_res +=
            (actual - fitted)
                .powi(2);

        ss_tot +=
            (actual - y_mean)
                .powi(2);
    }

    let r_squared =
        if ss_tot == 0.0 {
            f64::NAN
        } else {
            1.0 - ss_res / ss_tot
        };

    let result_matrix =
        |matrix: Matrix| {
            Value::Matrix(
                Rc::new(
                    RefCell::new(matrix)
                )
            )
        };

    let mut result =
        Object::new();

    result.set_type_name(
        "LinearRegressionResult"
    );

    result.set_field(
        "coefficients",
        result_matrix(coefficients),
    );

    result.set_field(
        "fitted",
        result_matrix(predicted),
    );

    result.set_field(
        "r_squared",
        Value::Float(r_squared),
    );

    result.set_field(
        "residual_sum_of_squares",
        Value::Float(ss_res),
    );

    Ok(Value::Object(
        Rc::new(
            RefCell::new(result)
        )
    ))
}