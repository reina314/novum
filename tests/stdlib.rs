use novum::{
    runtime::Value,
};

mod common;
use common::{
    run,
    unwrap_value,
    assert_int,
    assert_float,
    assert_bool,
    assert_list,
    assert_float_list,
    assert_string,
    assert_matrix,
    assert_error_kind,
};

#[test]
fn builtin_len() {
    assert_int(
        "len([1, 2, 3])",
        3,
    );
}

#[test]
fn builtin_typeof() {
    assert_string(
        "typeof(42)",
        "Int",
    );
}

#[test]
fn builtin_str() {
    assert_string(
        "str(42)",
        "42",
    );
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
    match run(
        r#"
        import fs
        fs.exists("__novum_file_that_does_not_exist__")
        "#
    ) {
        Ok(Value::Bool(false)) => {}

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn stdlib_process_cwd() {
    match run(
        r#"
        import process as p
        p.cwd()?
        "#
    ) {
        Ok(Value::Path(_)) => {}

        other => {
            panic!(
                "expected Str, got {other:?}"
            );
        }
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
        6,
    )
}

#[test]
fn matrix_matmul_2x2() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        let b =
            linalg.matrix([
                [5, 6],
                [7, 8]
            ])

        a @ b
        "#,
        &[
            &[19.0, 22.0],
            &[43.0, 50.0],
        ],
    );
}

#[test]
fn matrix_transpose() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        a.transpose()
        "#,
        &[
            &[1.0, 4.0],
            &[2.0, 5.0],
            &[3.0, 6.0],
        ],
    );
}

#[test]
fn matrix_inverse() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [4, 7],
                [2, 6]
            ])

        linalg.inverse(a)
        "#,
        &[
            &[0.6, -0.7],
            &[-0.2, 0.4],
        ],
    );
}

#[test]
fn matrix_vector_mul() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        let b =
            linalg.matrix([
                [5],
                [6]
            ])

        a @ b
        "#,
        &[
            &[17.0],
            &[39.0],
        ],
    );
}

#[test]
fn vector_matrix_mul() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2]
            ])

        let b =
            linalg.matrix([
                [3, 4],
                [5, 6]
            ])

        a @ b
        "#,
        &[
            &[13.0, 16.0],
        ],
    );
}

#[test]
fn matrix_shape() {
    assert_list(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        linalg.shape(a)
        "#,
        &[2, 3],
    );
}

#[test]
fn matrix_rows() {
    assert_int(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        linalg.rows(a)
        "#,
        2,
    );
}

#[test]
fn matrix_cols() {
    assert_int(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        linalg.cols(a)
        "#,
        3,
    );
}

#[test]
fn matrix_determinant() {
    assert_float(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        linalg.det(a)
        "#,
        -2.0,
    );
}

#[test]
fn vector_dot() {
    assert_float(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3]
            ])

        let b =
            linalg.matrix([
                [4],
                [5],
                [6]
            ])

        a @ b
        "#,
        32.0,
    );
}


