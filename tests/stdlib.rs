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
fn matrix_addition() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        let B = matrix([
            [5, 6],
            [7, 8]
        ])

        A + B
        "#,
        &[
            &[6.0, 8.0],
            &[10.0, 12.0],
        ],
    );
}

#[test]
fn matrix_subtraction() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [5, 6],
            [7, 8]
        ])

        let B = matrix([
            [1, 2],
            [3, 4]
        ])

        A - B
        "#,
        &[
            &[4.0, 4.0],
            &[4.0, 4.0],
        ],
    );
}

#[test]
fn matrix_scalar_multiplication() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        A * 2
        "#,
        &[
            &[2.0, 4.0],
            &[6.0, 8.0],
        ],
    );
}

#[test]
fn scalar_matrix_multiplication() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        3 * A
        "#,
        &[
            &[3.0, 6.0],
            &[9.0, 12.0],
        ],
    );
}

#[test]
fn matrix_elementwise_multiplication() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        let B = matrix([
            [5, 6],
            [7, 8]
        ])

        A * B
        "#,
        &[
            &[5.0, 12.0],
            &[21.0, 32.0],
        ],
    );
}

#[test]
fn matrix_matmul_2x2() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        let B = matrix([
            [5, 6],
            [7, 8]
        ])

        A @ B
        "#,
        &[
            &[19.0, 22.0],
            &[43.0, 50.0],
        ],
    );
}

#[test]
fn matrix_matmul_rectangular() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        let B = matrix([
            [7, 8],
            [9, 10],
            [11, 12]
        ])

        A @ B
        "#,
        &[
            &[58.0, 64.0],
            &[139.0, 154.0],
        ],
    );
}

#[test]
fn matrix_matmul_dimension_error() {
    assert_error_kind(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        let B = matrix([
            [1, 2, 3]
        ])

        A @ B
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn matrix_transpose() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A.transpose()
        "#,
        &[
            &[1.0, 4.0],
            &[2.0, 5.0],
            &[3.0, 6.0],
        ],
    );
}

#[test]
fn matrix_transpose_module_function() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        linalg.transpose(A)
        "#,
        &[
            &[1.0, 3.0],
            &[2.0, 4.0],
        ],
    );
}

#[test]
fn matrix_shape() {
    assert_list(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        linalg.shape(A)
        "#,
        &[2, 3],
    );
}

#[test]
fn matrix_rows() {
    assert_int(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        linalg.rows(A)
        "#,
        2,
    );
}

#[test]
fn matrix_cols() {
    assert_int(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        linalg.cols(A)
        "#,
        3,
    );
}

#[test]
fn matrix_property_rows() {
    assert_int(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A.rows
        "#,
        2,
    );
}

#[test]
fn matrix_property_cols() {
    assert_int(
        r#"
        import linalg

        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A.cols
        "#,
        3,
    );
}

#[test]
fn matrix_determinant() {
    assert_float(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        linalg.det(A)
        "#,
        -2.0,
    );
}

#[test]
fn matrix_determinant_3x3() {
    assert_float(
        r#"
        import linalg

        let A = matrix([
            [6, 1, 1],
            [4, -2, 5],
            [2, 8, 7]
        ])

        linalg.det(A)
        "#,
        -306.0,
    );
}

#[test]
fn matrix_inverse() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [4, 7],
            [2, 6]
        ])

        linalg.inverse(A)
        "#,
        &[
            &[0.6, -0.7],
            &[-0.2, 0.4],
        ],
    );
}

#[test]
fn matrix_inverse_round_trip() {
    assert_matrix(
        r#"
        import linalg

        let A = matrix([
            [4, 7],
            [2, 6]
        ])

        let inv =
            linalg.inverse(A)

        A @ inv
        "#,
        &[
            &[1.0, 0.0],
            &[0.0, 1.0],
        ],
    );
}

#[test]
fn matrix_inverse_singular_error() {
    assert_error_kind(
        r#"
        import linalg

        let A = matrix([
            [1, 2],
            [2, 4]
        ])

        linalg.inverse(A)
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn matrix_index() {
    assert_float(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A[1, 2]
        "#,
        6.0,
    );
}

#[test]
fn matrix_index_first_element() {
    assert_float(
        r#"
        let A = matrix([
            [10, 20],
            [30, 40]
        ])

        A[0, 0]
        "#,
        10.0,
    );
}

#[test]
fn matrix_index_out_of_bounds() {
    assert_error_kind(
        r#"
        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        A[2, 0]
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn matrix_assignment() {
    assert_float(
        r#"
        let A = matrix([
            [1, 2],
            [3, 4]
        ])

        A[1, 0] = 99

        A[1, 0]
        "#,
        99.0,
    );
}

#[test]
fn matrix_slice() {
    assert_matrix(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ])

        A[0..2, 1..3]
        "#,
        &[
            &[2.0, 3.0],
            &[5.0, 6.0],
        ],
    );
}

#[test]
fn matrix_all_rows_slice() {
    assert_matrix(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ])

        A[.., 1..3]
        "#,
        &[
            &[2.0, 3.0],
            &[5.0, 6.0],
            &[8.0, 9.0],
        ],
    );
}

#[test]
fn matrix_single_row_slice() {
    assert_matrix(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A[0, 1..3]
        "#,
        &[
            &[2.0, 3.0],
        ],
    );
}

#[test]
fn matrix_single_column_slice() {
    assert_matrix(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ])

        A[0..2, 1]
        "#,
        &[
            &[2.0],
            &[5.0],
        ],
    );
}

#[test]
fn list_vector() {
    assert_vector(
        r#"
        [1, 2, 3].vector()
        "#,
        &[1.0, 2.0, 3.0],
    );
}

#[test]
fn vector_addition() {
    assert_vector(
        r#"
        [1, 2, 3].vector()
        +
        [4, 5, 6].vector()
        "#,
        &[5.0, 7.0, 9.0],
    );
}

#[test]
fn vector_subtraction() {
    assert_vector(
        r#"
        [4, 5, 6].vector()
        -
        [1, 2, 3].vector()
        "#,
        &[3.0, 3.0, 3.0],
    );
}

#[test]
fn vector_scalar_multiplication() {
    assert_vector(
        r#"
        [1, 2, 3].vector()
        * 2
        "#,
        &[2.0, 4.0, 6.0],
    );
}

#[test]
fn scalar_vector_multiplication() {
    assert_vector(
        r#"
        3 *
        [1, 2, 3].vector()
        "#,
        &[3.0, 6.0, 9.0],
    );
}

#[test]
fn vector_dot() {
    assert_float(
        r#"
        [1, 2, 3]
            .vector()
            .dot(
                [4, 5, 6]
                    .vector()
            )
        "#,
        32.0,
    );
}

#[test]
fn vector_norm() {
    assert_float(
        r#"
        [3, 4]
            .vector()
            .norm()
        "#,
        5.0,
    );
}

#[test]
fn vector_matrix_multiplication() {
    assert_vector(
        r#"
        let v =
            [1, 2].vector()

        let A =
            matrix([
                [3, 4],
                [5, 6]
            ])

        v @ A
        "#,
        &[13.0, 16.0],
    );
}

#[test]
fn matrix_vector_multiplication() {
    assert_vector(
        r#"
        let A =
            matrix([
                [1, 2],
                [3, 4]
            ])

        let v =
            [5, 6].vector()

        A @ v
        "#,
        &[17.0, 39.0],
    );
}

#[test]
fn vector_matrix_dimension_error() {
    assert_error_kind(
        r#"
        let v =
            [1, 2, 3].vector()

        let A =
            matrix([
                [1, 2],
                [3, 4]
            ])

        v @ A
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn matrix_vector_dimension_error() {
    assert_error_kind(
        r#"
        let A =
            matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        let v =
            [1, 2].vector()

        A @ v
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn vector_equality() {
    common::assert_bool(
        r#"
        [1, 2, 3].vector()
        ==
        [1, 2, 3].vector()
        "#,
        true,
    );
}

#[test]
fn vector_inequality() {
    common::assert_bool(
        r#"
        [1, 2, 3].vector()
        ==
        [1, 2, 4].vector()
        "#,
        false,
    );
}

#[test]
fn matrix_equality() {
    common::assert_bool(
        r#"
        matrix([
            [1, 2],
            [3, 4]
        ])
        ==
        matrix([
            [1, 2],
            [3, 4]
        ])
        "#,
        true,
    );
}

#[test]
fn matrix_inequality() {
    common::assert_bool(
        r#"
        matrix([
            [1, 2],
            [3, 4]
        ])
        ==
        matrix([
            [1, 2],
            [3, 5]
        ])
        "#,
        false,
    );
}

#[test]
fn matrix_property_shape() {
    assert_list(
        r#"
        import linalg

        let A =
            matrix([
                [1, 2, 3],
                [4, 5, 6]
            ])

        A.shape
        "#,
        &[2, 3],
    );
}




