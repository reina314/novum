use novum::{
    runtime::Value,
    error::ErrorKind,
};

mod common;
use common::{
    run,
    assert_int,
    assert_float,
    assert_bool,
    assert_list,
    assert_string,
    assert_vector,
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
fn linalg_matrix() {
    assert_matrix(
        r#"
        import linalg

        linalg.matrix([
            [1, 2],
            [3, 4]
        ])
        "#,
        &[
            &[1.0, 2.0],
            &[3.0, 4.0],
        ],
    );
}

#[test]
fn linalg_vector() {
    assert_vector(
        r#"
        import linalg

        linalg.vector([
            1,
            2,
            3
        ])
        "#,
        &[
            1.0,
            2.0,
            3.0,
        ],
    );
}

#[test]
fn linalg_matrix_addition() {
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

        a + b
        "#,
        &[
            &[6.0, 8.0],
            &[10.0, 12.0],
        ],
    );
}

#[test]
fn linalg_matrix_subtraction() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [5, 6],
                [7, 8]
            ])

        let b =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        a - b
        "#,
        &[
            &[4.0, 4.0],
            &[4.0, 4.0],
        ],
    );
}

#[test]
fn linalg_matrix_elementwise_multiplication() {
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

        a * b
        "#,
        &[
            &[5.0, 12.0],
            &[21.0, 32.0],
        ],
    );
}

#[test]
fn linalg_matrix_scalar_multiplication() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        a * 2
        "#,
        &[
            &[2.0, 4.0],
            &[6.0, 8.0],
        ],
    );
}

#[test]
fn linalg_scalar_matrix_multiplication() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        3 * a
        "#,
        &[
            &[3.0, 6.0],
            &[9.0, 12.0],
        ],
    );
}

#[test]
fn linalg_matrix_matmul() {
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
fn linalg_matrix_matmul_rectangular() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        let b =
            linalg.matrix([
                [7, 8],
                [9, 10],
                [11, 12]
            ])

        a @ b
        "#,
        &[
            &[58.0, 64.0],
            &[139.0, 154.0],
        ],
    );
}

#[test]
fn linalg_matrix_transpose() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        linalg.transpose(a)
        "#,
        &[
            &[1.0, 4.0],
            &[2.0, 5.0],
            &[3.0, 6.0],
        ],
    );
}

#[test]
fn linalg_matrix_determinant() {
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
fn linalg_matrix_determinant_3x3() {
    assert_float(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [6, 1, 1],
                [4, -2, 5],
                [2, 8, 7]
            ])

        linalg.det(a)
        "#,
        -306.0,
    );
}

#[test]
fn linalg_matrix_inverse() {
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
fn linalg_matrix_inverse_round_trip() {
    assert_matrix(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [4, 7],
                [2, 6]
            ])

        let inv =
            linalg.inverse(a)

        a @ inv
        "#,
        &[
            &[1.0, 0.0],
            &[0.0, 1.0],
        ],
    );
}

#[test]
fn linalg_matrix_shape() {
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
fn linalg_matrix_rows() {
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
fn linalg_matrix_cols() {
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
fn linalg_vector_addition() {
    assert_vector(
        r#"
        import linalg

        let a =
            linalg.vector([
                1,
                2,
                3
            ])

        let b =
            linalg.vector([
                4,
                5,
                6
            ])

        a + b
        "#,
        &[
            5.0,
            7.0,
            9.0,
        ],
    );
}

#[test]
fn linalg_vector_subtraction() {
    assert_vector(
        r#"
        import linalg

        let a =
            linalg.vector([
                5,
                6,
                7
            ])

        let b =
            linalg.vector([
                1,
                2,
                3
            ])

        a - b
        "#,
        &[
            4.0,
            4.0,
            4.0,
        ],
    );
}

#[test]
fn linalg_vector_scalar_multiplication() {
    assert_vector(
        r#"
        import linalg

        let a =
            linalg.vector([
                1,
                2,
                3
            ])

        a * 2
        "#,
        &[
            2.0,
            4.0,
            6.0,
        ],
    );
}

#[test]
fn linalg_vector_dot_product() {
    assert_float(
        r#"
        import linalg

        let a =
            linalg.vector([
                1,
                2,
                3
            ])

        let b =
            linalg.vector([
                4,
                5,
                6
            ])

        a @ b
        "#,
        32.0,
    );
}

#[test]
fn linalg_matrix_vector_multiplication() {
    assert_vector(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        let v =
            linalg.vector([
                5,
                6
            ])

        a @ v
        "#,
        &[
            17.0,
            39.0,
        ],
    );
}

#[test]
fn linalg_vector_matrix_multiplication() {
    assert_vector(
        r#"
        import linalg

        let v =
            linalg.vector([
                1,
                2
            ])

        let a =
            linalg.matrix([
                [3, 4],
                [5, 6]
            ])

        v @ a
        "#,
        &[
            13.0,
            16.0,
        ],
    );
}

#[test]
fn linalg_matrix_dimension_error() {
    assert_error_kind(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        let b =
            linalg.matrix([
                [1, 2, 3]
            ])

        a @ b
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_vector_dimension_error() {
    assert_error_kind(
        r#"
        import linalg

        let a =
            linalg.vector([
                1,
                2,
                3
            ])

        let b =
            linalg.vector([
                4,
                5
            ])

        a @ b
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_matrix_addition_dimension_error() {
    assert_error_kind(
        r#"
        import linalg

        let a =
            linalg.matrix([
                [1, 2]
            ])

        let b =
            linalg.matrix([
                [1, 2],
                [3, 4]
            ])

        a + b
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_vector_rejects_non_numeric_value() {
    assert_error_kind(
        r#"
        import linalg

        linalg.vector([
            1,
            "hello",
            3
        ])
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_vector_requires_list() {
    assert_error_kind(
        r#"
        import linalg

        linalg.vector(1)
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_matrix_requires_list() {
    assert_error_kind(
        r#"
        import linalg

        linalg.matrix(1)
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_matrix_rejects_non_numeric_value() {
    assert_error_kind(
        r#"
        import linalg

        linalg.matrix([
            [1, 2],
            [3, "hello"]
        ])
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_matrix_rejects_ragged_rows() {
    assert_error_kind(
        r#"
        import linalg

        linalg.matrix([
            [1, 2],
            [3]
        ])
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn linalg_linear_regression() {
    assert_matrix(
        r#"
        import linalg

        let X =
            linalg.matrix([
                [1, 1],
                [1, 2],
                [1, 3],
                [1, 4]
            ])

        let y =
            linalg.matrix([
                [2],
                [4],
                [6],
                [8]
            ])

        linalg.linear_regression(X, y)["coefficients"]
        "#,
        &[
            &[0.0],
            &[2.0],
        ],
    );
}



