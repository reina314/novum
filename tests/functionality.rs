mod common;

use common::{
    run,
    assert_float_close,
};
use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{Value, Object, Matrix};

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
fn csv_read() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/simple.csv"
            ).nrows
            "#
        );

    assert_eq!(
        result,
        Value::Int(3)
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
fn nested_module_namespace() {
    let source = r#"
        import tests.modules.a

        tests.modules.a.get_b(5)
    "#;

    // run...
}


