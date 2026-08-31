use novum::runtime::Value;

mod common;
use common::{assert_bool, assert_float, assert_int, assert_string, run};

#[test]
fn builtin_len() {
    assert_int("len([1, 2, 3])", 3);
}

#[test]
fn builtin_typeof() {
    assert_string("typeof(42)", "Int");
}

#[test]
fn builtin_str() {
    assert_string("str(42)", "42");
}

#[test]
fn stdlib_math_sqrt() {
    assert_float(
        r#"
        import math as m
        m.sqrt(16)
        "#,
        4.0,
    );
}

#[test]
fn stdlib_math_sin() {
    assert_float(
        r#"
        import math
        math.sin(math.pi())
        "#,
        0.0,
    );
}

#[test]
fn stdlib_fs_exists() {
    match run(r#"
        import fs
        fs.exists("__novum_file_that_does_not_exist__")
        "#)
    {
        Ok(Value::Bool(false)) => {},

        other => {
            panic!("unexpected result: {other:?}");
        },
    }
}

#[test]
fn stdlib_process_cwd() {
    match run(r#"
        import process as p
        p.cwd()?
        "#)
    {
        Ok(Value::Path(_)) => {},

        other => {
            panic!("expected Str, got {other:?}");
        },
    }
}

#[test]
fn process_cwd_returns_path() {
    assert_string(
        r#"
        import process

        let result =
            process.cwd()

        match result {
            Ok(p) => typeof(p),
            Err(_) => "error"
        }
        "#,
        "Path",
    );
}

#[test]
fn process_cwd_path_methods() {
    assert_bool(
        r#"
        import process

        let result =
            process.cwd()

        match result {
            Ok(p) => p.is_dir(),
            Err(_) => false
        }
        "#,
        true,
    );
}

#[test]
fn vm_csv_read() {
    assert_int(
        r#"
        import csv

        csv.read(
            "tests/data/experiment.csv"
        ).nrows
        "#,
        122,
    )
}
