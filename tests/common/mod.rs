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
            assert!(
                (actual - expected).abs()
                    < 1e-10,
                "expected Float({expected}), got {actual:?}\nsource:\n{source}"
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
