use std::rc::Rc;

use novum::{
    Lexer,
    Parser,
    Error,
    ErrorKind,
    runtime::Value,
    vm::{
        Compiler,
        Vm,
    }
};

pub fn run(
    source: &str,
) -> Result<Value, Error> {
    let tokens =
        Lexer::new(source)
            .lex()?;

    let mut parser =
        Parser::new(tokens);

    let program =
        parser.parse()?;

    let chunk =
        Compiler::new()
            .compile(&program)?;

    let mut vm =
        Vm::new();

    vm.run(
        Rc::new(chunk)
    )
}

pub fn unwrap_value(
    source: &str,
) -> Value {
    match run(source) {
        Ok(value) =>
            value,

        Err(error) => {
            panic!(
                "unexpected runtime error:\n{error:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_int(
    source: &str,
    expected: i64,
) {
    match unwrap_value(source) {
        Value::Int(actual) => {
            assert_eq!(
                actual,
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Int({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_float(
    source: &str,
    expected: f64,
) {
    match unwrap_value(source) {
        Value::Float(actual) => {
            let abs_error =
                (actual - expected).abs();

            let tolerance =
                1e-10_f64
                    .max(
                        expected.abs()
                            * 1e-10
                    );

            assert!(
                abs_error <= tolerance,
                "expected Float({expected}), got {actual}, error={abs_error}\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Float({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_bool(
    source: &str,
    expected: bool,
) {
    match unwrap_value(source) {
        Value::Bool(actual) => {
            assert_eq!(
                actual,
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Bool({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_string(
    source: &str,
    expected: &str,
) {
    match unwrap_value(source) {
        Value::Str(actual) => {
            assert_eq!(
                actual.as_str(),
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Str({expected:?}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_list(
    source: &str,
    expected: &[i64],
) {
    match unwrap_value(source) {
        Value::List(list) => {
            assert_eq!(
                list.len(),
                expected.len(),
                "\nsource:\n{source}"
            );

            for (
                index,
                expected_value,
            ) in expected.iter().enumerate()
            {
                let actual =
                    list.get(index)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing list element at index {index}\nsource:\n{source}"
                            )
                        });

                match actual {
                    Value::Int(actual_value) => {
                        assert_eq!(
                            actual_value,
                            *expected_value,
                            "element {index}\nsource:\n{source}"
                        );
                    }

                    other => {
                        panic!(
                            "expected Int at index {index}, got {other:?}\nsource:\n{source}"
                        );
                    }
                }
            }
        }

        actual => {
            panic!(
                "expected List, got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_float_list(
    source: &str,
    expected: &[f64],
) {
    match unwrap_value(source) {
        Value::List(list) => {
            assert_eq!(
                list.len(),
                expected.len(),
                "\nsource:\n{source}"
            );

            for (
                index,
                expected_value,
            ) in expected.iter().enumerate()
            {
                let actual =
                    list.get(index)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing list element at index {index}\nsource:\n{source}"
                            )
                        });

                match actual {
                    Value::Float(actual_value) => {
                        assert!(
                            (actual_value - expected_value).abs()
                                < 1e-10,
                            "element {index}: expected Float({expected_value}), got {actual_value}\nsource:\n{source}"
                        );
                    }

                    Value::Int(actual_value) => {
                        assert!(
                            (actual_value as f64 - expected_value).abs()
                                < 1e-10,
                            "element {index}: expected numeric value {expected_value}, got Int({actual_value})\nsource:\n{source}"
                        );
                    }

                    other => {
                        panic!(
                            "expected numeric value at index {index}, got {other:?}\nsource:\n{source}"
                        );
                    }
                }
            }
        }

        actual => {
            panic!(
                "expected List, got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_matrix(
    source: &str,
    expected: &[&[f64]],
) {
    match unwrap_value(source) {
        Value::Matrix(matrix) => {
            let matrix =
                matrix.borrow();

            assert_eq!(
                matrix.rows(),
                expected.len(),
                "row count mismatch\nsource:\n{source}"
            );

            let expected_cols =
                expected
                    .first()
                    .map(|row| row.len())
                    .unwrap_or(0);

            assert_eq!(
                matrix.cols(),
                expected_cols,
                "column count mismatch\nsource:\n{source}"
            );

            for row in 0..expected.len() {
                assert_eq!(
                    expected[row].len(),
                    expected_cols,
                    "expected matrix is not rectangular\nsource:\n{source}"
                );

                for col in 0..expected_cols {
                    let actual =
                        matrix
                            .get(row, col)
                            .unwrap_or_else(|| {
                                panic!(
                                    "missing matrix element ({row}, {col})\nsource:\n{source}"
                                )
                            });

                    let expected_value =
                        expected[row][col];

                    assert!(
                        (actual - expected_value).abs()
                            < 1e-10,
                        "matrix element ({row}, {col}): expected {expected_value}, got {actual}\nsource:\n{source}"
                    );
                }
            }
        }

        actual => {
            panic!(
                "expected Matrix, got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_vector(
    source: &str,
    expected: &[f64],
) {
    match unwrap_value(source) {
        Value::Vector(vector) => {
            let vector =
                vector.borrow();

            assert_eq!(
                vector.len(),
                expected.len(),
                "vector length mismatch\nsource:\n{source}"
            );

            for (
                index,
                expected_value,
            ) in expected.iter().enumerate()
            {
                let actual =
                    vector
                        .get(index)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing vector element at index {index}\nsource:\n{source}"
                            )
                        });

                assert!(
                    (
                        actual
                        - expected_value
                    ).abs()
                        < 1e-10,
                    "vector element {index}: expected {expected_value}, got {actual}\nsource:\n{source}"
                );
            }
        }

        actual => {
            panic!(
                "expected Vector, got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

pub fn assert_error_kind(
    source: &str,
    expected: ErrorKind,
) {
    match run(source) {
        Ok(value) => {
            panic!(
                "expected {expected:?} error, got {value:?}\nsource:\n{source}"
            );
        }

        Err(error) => {
            assert_eq!(
                error.kind,
                expected,
                "expected error kind {expected:?}, got {error:?}\nsource:\n{source}"
            );
        }
    }
}