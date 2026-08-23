mod common;

use common::{
    run,
    run_result,
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
fn import_stdlib_alias() {
    let result =
        run(
            r#"
            import math as m

            m.sqrt(16)
            "#
        );

    assert_eq!(
        result,
        Value::Float(4.0)
    );
}

#[test]
fn import_user_module_alias() {
    let result =
        run(
            r#"
            import tests.modules.counter as mod

            mod.value
            "#
        );

    assert_eq!(
        result,
        Value::Int(1)
    );
}

#[test]
fn import_alias_does_not_bind_original_name() {
    let result =
        run_result(
            r#"
            import math as m

            math.sqrt(4)
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn import_without_alias_keeps_namespace() {
    let result =
        run(
            r#"
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

#[test]
fn public_let_is_exported() {
    // fixture:
    //
    // pub let answer = 42
    //
    let result =
        run(
            r#"
            import tests.modules.visibility
            tests.modules.visibility.answer
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn private_let_is_hidden() {
    let result =
        run_result(
            r#"
            import tests.modules.visibility
            tests.modules.visibility.secret
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn public_lambda_is_exported() {
    let result =
        run(
            r#"
            import tests.modules.visibility
            tests.modules.visibility.add(2, 3)
            "#
        );

    assert_eq!(
        result,
        Value::Int(5)
    );
}

#[test]
fn private_lambda_is_hidden() {
    let result =
        run_result(
            r#"
            import tests.modules.visibility
            tests.modules.visibility.helper(10)
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn pub_local_is_error() {
    let result =
        run_result(
            r#"
            {
                pub let x = 10
                x
            }
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn path_api() {
    let result =
        run(
            r#"
            let p =
                path("data/result.csv")

            p.name()?
                "#
        );

    // "result.csv"
}

#[test]
fn path_extension() {
    let result =
        run(
            r#"
            path("data/result.csv")
                .extension()?
            "#
        );

    // "csv"
}

#[test]
fn path_stem() {
    let result =
        run(
            r#"
            path("data/result.csv")
                .stem()?
            "#
        );

    // "result"
}

#[test]
fn path_parent() {
    let result =
        run(
            r#"
            path("data/result.csv")
                .parent()?
                .to_str()
            "#
        );

    // "data"
}

#[test]
fn path_join() {
    let result =
        run(
            r#"
            path("data")
                .join("result.csv")
                .to_str()
            "#
        );

    // OS-dependent path representation
}

