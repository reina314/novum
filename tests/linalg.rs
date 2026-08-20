mod common;

use common::{
    run,
    assert_float_close,
};
use novum::runtime::{Value, Matrix};

#[test]
fn matrix_addition() {
    assert_eq!(
        run(
            r#"
            import linalg

            let A = matrix([
                [1, 2],
                [3, 4]
            ]);

            let B = matrix([
                [5, 6],
                [7, 8]
            ]);

            linalg.det(A)
            "#
        ),
        Value::Float(-2.0)
    );
}

#[test]
fn matmul_result() {
    let a = Matrix::from_rows(vec![
        vec![1.0, 2.0],
        vec![3.0, 4.0],
    ]).unwrap();

    let b = Matrix::from_rows(vec![
        vec![5.0, 6.0],
        vec![7.0, 8.0],
    ]).unwrap();

    let expected = Matrix::from_rows(vec![
        vec![19.0, 22.0],
        vec![43.0, 50.0],
    ]).unwrap();

    let actual = a.matmul(&b).unwrap();

    assert!(
        actual.approx_eq(
            &expected,
            1e-10,
            1e-10,
        )
    );
}

#[test]
fn matrix_transpose() {
    assert_eq!(
        run(
            r#"
            import linalg

            let A = matrix([
                [1, 2],
                [3, 4]
            ]);

            let B = linalg.transpose(A);

            linalg.det(B)
            "#
        ),
        Value::Float(-2.0)
    );
}

#[test]
fn matrix_scalar_mul() {
    assert_eq!(
        run(
            r#"
            import linalg

            let A = matrix([
                [1, 2],
                [3, 4]
            ]);

            let B = A * 2;

            linalg.det(B)
            "#
        ),
        Value::Float(-8.0)
    );
}

#[test]
fn matrix_arbitrary_shape() {
    let source = r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9],
            [10, 11, 12]
        ]);

        A[3, 2]
    "#;

    assert_float_close(
        match run(source) {
            Value::Float(x) => x,
            other => panic!(
                "expected Float, got {:?}",
                other
            ),
        },
        12.0,
    );
}

#[test]
fn matrix_assignment() {
    let source = r#"
        let A = matrix([
            [1, 2],
            [3, 4]
        ]);

        A[1, 0] = 99;

        A[1, 0]
    "#;

    assert_float_close(
        match run(source) {
            Value::Float(x) => x,
            other => panic!(
                "expected Float, got {:?}",
                other
            ),
        },
        99.0,
    );
}

#[test]
fn matrix_index() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ]);

        A[1, 2]
        "#
    );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                6.0,
            );
        }

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn matrix_index_first_element() {
    let result = run(
        r#"
        let A = matrix([
            [10, 20],
            [30, 40]
        ]);

        A[0, 0]
        "#
    );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                10.0,
            );
        }

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn matrix_slice() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ]);

        A[0..2, 1..3]
        "#
    );

    let expected =
        Matrix::from_rows(vec![
            vec![2.0, 3.0],
            vec![5.0, 6.0],
        ])
        .unwrap();

    match result {
        Value::Matrix(actual) => {
            assert!(
                actual.borrow()
                    .approx_eq(
                        &expected,
                        1e-10,
                        1e-10,
                    )
            );
        }

        other => {
            panic!(
                "expected Matrix, got {:?}",
                other
            );
        }
    }
}

#[test]
fn matrix_all_rows() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ]);

        A[.., 1..3]
        "#
    );

    let expected =
        Matrix::from_rows(vec![
            vec![2.0, 3.0],
            vec![5.0, 6.0],
            vec![8.0, 9.0],
        ])
        .unwrap();

    match result {
        Value::Matrix(actual) => {
            assert!(
                actual.borrow()
                    .approx_eq(
                        &expected,
                        1e-10,
                        1e-10,
                    )
            );
        }

        other => panic!(
            "expected Matrix, got {:?}",
            other
        ),
    }
}

#[test]
fn matrix_single_row_slice() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ]);

        A[0, 1..3]
        "#
    );

    let expected =
        Matrix::from_rows(vec![
            vec![2.0, 3.0],
        ])
        .unwrap();

    match result {
        Value::Matrix(actual) => {
            assert!(
                actual.borrow()
                    .approx_eq(
                        &expected,
                        1e-10,
                        1e-10,
                    )
            );
        }

        other => panic!(
            "expected Matrix, got {:?}",
            other
        ),
    }
}

#[test]
fn matrix_single_column_slice() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2, 3],
            [4, 5, 6]
        ]);

        A[0..2, 1]
        "#
    );

    let expected =
        Matrix::from_rows(vec![
            vec![2.0],
            vec![5.0],
        ])
        .unwrap();

    match result {
        Value::Matrix(actual) => {
            assert!(
                actual.borrow()
                    .approx_eq(
                        &expected,
                        1e-10,
                        1e-10,
                    )
            );
        }

        other => panic!(
            "expected Matrix, got {:?}",
            other
        ),
    }
}

#[test]
fn matrix_scalar_index_still_works() {
    let result = run(
        r#"
        let A = matrix([
            [1, 2],
            [3, 4]
        ]);

        A[1, 0]
        "#
    );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                3.0,
            );
        }

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn linear_regression() {
    let result =
        run(
            r#"
            import linalg

            let X = matrix([
                [1, 1],
                [1, 2],
                [1, 3],
                [1, 4]
            ]);

            let y = matrix([
                [2],
                [4],
                [6],
                [8]
            ]);

            linalg.linear_regression(X, y)
            "#
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            let coefficients =
                match object
                    .get_field("coefficients")
                    .unwrap()
                {
                    Value::Matrix(matrix) => matrix,
                    _ => panic!("invalid coefficients"),
                };

            let matrix =
                coefficients.borrow();

            assert_float_close(
                matrix.get(0, 0).unwrap(),
                0.0,
            );

            assert_float_close(
                matrix.get(1, 0).unwrap(),
                2.0,
            );

            assert_float_close(
                match object
                    .get_field("r_squared")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid R²"),
                },
                1.0,
            );
        }

        other => panic!(
            "expected Object, got {:?}",
            other
        ),
    }
}

