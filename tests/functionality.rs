mod common;

use common::{
    run,
    assert_float_close,
};
use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{Value};

#[test]
fn import_stats_module() {
    let result =
        run(
            r#"
            import stats

            stats.mean([
                1, 2, 3, 4, 5
            ])
            "#
        );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                3.0,
            );
        }

        other => {
            panic!(
                "expected Float, got {:?}",
                other
            );
        }
    }
}

#[test]
fn import_linalg_module() {
    let result =
        run(
            r#"
            import linalg

            let A =
                linalg.matrix([
                    [1, 2],
                    [3, 4]
                ])

            linalg.det(A)
            "#
        );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                -2.0,
            );
        }

        other => {
            panic!(
                "expected Float, got {:?}",
                other
            );
        }
    }
}

#[test]
fn import_math_module() {
    let result =
        run(
            r#"
            import math

            math.sqrt(9)
            "#
        );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                3.0,
            );
        }

        other => {
            panic!(
                "expected Float, got {:?}",
                other
            );
        }
    }
}

#[test]
fn import_unknown_module_is_error() {
    let source =
        "import does_not_exist";

    let mut lexer =
        Lexer::new(source);

    let tokens =
        lexer.lex().unwrap();

    let mut parser =
        Parser::new(tokens);

    let program =
        parser.parse().unwrap();

    let mut interpreter =
        Interpreter::new();

    assert!(
        interpreter
            .eval_program(&program)
            .is_err()
    );
}

#[test]
fn cyclic_import_is_error() {
    let source =
        "import tests.modules.c";

    let mut lexer =
        Lexer::new(source);

    let tokens =
        lexer.lex().unwrap();

    let mut parser =
        Parser::new(tokens);

    let program =
        parser.parse().unwrap();

    let mut interpreter =
        Interpreter::new();

    assert!(
        interpreter
            .eval_program(&program)
            .is_err()
    );
}

#[test]
fn importing_same_module_twice_uses_cache() {
    let result =
        run(
            r#"
            import tests.modules.counter
            import tests.modules.counter

            tests.modules.counter.value
            "#
        );

    assert_eq!(
        result,
        Value::Int(1)
    );
}

#[test]
fn builtin_is_available_without_import() {
    let result =
        run(
            r#"
            input
            "#
        );

    // adapt to the actual builtin representation
    match result {
        Value::Builtin(_) => {}

        other => {
            panic!(
                "expected builtin, got {:?}",
                other
            );
        }
    }
}



