use novum::error::ErrorKind;

mod common;
use common::{
    assert_bool, assert_error_kind, assert_float, assert_int, assert_list, assert_matrix,
    assert_vector,
};

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
        &[&[1.0, 2.0], &[3.0, 4.0]],
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
        &[1.0, 2.0, 3.0],
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
        &[&[6.0, 8.0], &[10.0, 12.0]],
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
        &[&[4.0, 4.0], &[4.0, 4.0]],
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
        &[&[5.0, 12.0], &[21.0, 32.0]],
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
        &[&[2.0, 4.0], &[6.0, 8.0]],
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
        &[&[3.0, 6.0], &[9.0, 12.0]],
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
        &[&[19.0, 22.0], &[43.0, 50.0]],
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
        &[&[58.0, 64.0], &[139.0, 154.0]],
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
        &[&[1.0, 4.0], &[2.0, 5.0], &[3.0, 6.0]],
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
        &[&[0.6, -0.7], &[-0.2, 0.4]],
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
        &[&[1.0, 0.0], &[0.0, 1.0]],
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
        &[5.0, 7.0, 9.0],
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
        &[4.0, 4.0, 4.0],
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
        &[2.0, 4.0, 6.0],
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
        &[17.0, 39.0],
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
        &[13.0, 16.0],
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
        &[&[0.0], &[2.0]],
    );
}

#[test]
fn linalg_solve_matrix() {
    assert_matrix(
        r#"
        import linalg

        let A =
            linalg.matrix([
                [3, 1],
                [1, 2]
            ])

        let b =
            linalg.matrix([
                [9],
                [8]
            ])

        linalg.solve(A, b)
        "#,
        &[&[2.0], &[3.0]],
    );
}

#[test]
fn linalg_solve_vector() {
    assert_vector(
        r#"
        import linalg

        let A =
            linalg.matrix([
                [3, 1],
                [1, 2]
            ])

        let b =
            linalg.vector([
                9,
                8
            ])

        linalg.solve(A, b)
        "#,
        &[2.0, 3.0],
    );
}

#[test]
fn linalg_solve_lstsq() {
    assert_vector(
        r#"
        import linalg

        let A =
            linalg.matrix([
                [1],
                [2],
                [3],
                [4]
            ])

        let b =
            linalg.vector([
                2,
                4,
                6,
                8
            ])

        linalg.solve_lstsq(A, b)
        "#,
        &[2.0],
    );
}

#[test]
fn vm_ufcs_linalg() {
    assert_bool(
        r#"
        let A = [
            [1, 2],
            [3, 4]
        ].matrix()

        A.det() == -2.0
        "#,
        true,
    );

    assert_list(
        r#"
        let A = [
            [1, 2],
            [3, 4]
        ].matrix()

        A.shape()
        "#,
        &[2, 2],
    );
}
