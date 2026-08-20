mod common;

use common::{
    run,
    assert_float_close,
};
use novum::runtime::{Value};


#[test]
fn descriptive_statistics() {
    assert_float_close(
        match run("mean([1,2,3,4,5])") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        3.0,
    );

    assert_float_close(
        match run("variance([1,2,3,4,5])") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        2.0,
    );

    assert_float_close(
        match run("variance([1,2,3,4,5], true)") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        2.5,
    );

    assert_float_close(
        match run("median([1,5,2,4,3])") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        3.0,
    );
}

#[test]
fn quantile() {
    assert_float_close(
        match run("quantile([1,2,3,4,5], 0.5)") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        3.0,
    );

    assert_float_close(
        match run("quantile([1,2,3,4,5], 0.25)") {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        2.0,
    );
}

#[test]
fn pearson() {
    assert_float_close(
        match run(
            "pearson([1,2,3,4,5], [2,4,6,8,10])"
        ) {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        1.0,
    );
}

#[test]
fn spearman() {
    assert_float_close(
        match run(
            "spearman([10,20,30,40], [1,2,3,4])"
        ) {
            Value::Float(v) => v,
            other => panic!("expected Float, got {:?}", other),
        },
        1.0,
    );
}

#[test]
fn one_sample_t_zero() {
    let result =
        run(
            "one_sample_t([1,2,3,4,5], 3)"
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            assert_float_close(
                match object.get_field("statistic").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                },
                0.0,
            );

            assert_float_close(
                match object.get_field("p_value").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
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

#[test]
fn one_sample_t_zero_variance_at_null() {
    let result =
        run(
            "one_sample_t([5,5,5], 5)"
        );

    match result {
        Value::Object(object) => {
            let object = object.borrow();

            assert_float_close(
                match object
                    .get_field("statistic")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                },
                0.0,
            );

            assert_float_close(
                match object
                    .get_field("p_value")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
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

#[test]
fn one_sample_t_zero_variance_away_from_null() {
    let result =
        run(
            "one_sample_t([5,5,5], 0)"
        );

    match result {
        Value::Object(object) => {
            let object = object.borrow();

            let statistic =
                match object
                    .get_field("statistic")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                };

            let p =
                match object
                    .get_field("p_value")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
                };

            assert!(statistic.is_infinite());
            assert_float_close(p, 0.0);
        }

        other => panic!(
            "expected Object, got {:?}",
            other
        ),
    }
}

#[test]
fn paired_t_identical_samples() {
    let result =
        run(
            "paired_t([1,2,3], [1,2,3])"
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            assert_float_close(
                match object.get_field("statistic").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                },
                0.0,
            );

            assert_float_close(
                match object.get_field("p_value").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
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

#[test]
fn welch_t_identical_samples() {
    let result =
        run(
            "welch_t([1,2,3,4,5], [1,2,3,4,5])"
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            assert_float_close(
                match object.get_field("statistic").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                },
                0.0,
            );

            assert_float_close(
                match object.get_field("p_value").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
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

#[test]
fn mann_whitney_identical_samples() {
    let result =
        run(
            "mann_whitney([1,2,3,4], [1,2,3,4])"
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            let p =
                match object.get_field("p_value").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
                };

            assert!(p > 0.9);
        }

        other => panic!(
            "expected Object, got {:?}",
            other
        ),
    }
}

#[test]
fn anova_identical_groups() {
    let result = run(
        r#"
        anova([
            [1,2,3],
            [1,2,3],
            [1,2,3]
        ])
        "#
    );

    match result {
        Value::Object(object) => {
            let object = object.borrow();

            let p =
                match object.get_field("p_value").unwrap() {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
                };

            assert!(p > 0.99);
        }

        other => panic!(
            "expected Object, got {:?}",
            other
        ),
    }
}

#[test]
fn chi_square_test() {
    let result =
        run(
            r#"
            import csv
            import stats

            let df =
                csv.read(
                    "tests/data/categorical.csv"
                )

            let table =
                df.crosstab(
                    "condition",
                    "outcome"
                )

            stats.chi_square(table)
            "#
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            let statistic =
                match object
                    .get_field("statistic")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    other => panic!(
                        "unexpected statistic: {:?}",
                        other
                    ),
                };

            assert_float_close(
                statistic,
                2.0 / 3.0,
            );

            assert_eq!(
                object
                    .get_field(
                        "degrees_of_freedom"
                    ),
                Some(
                    Value::Int(1)
                )
            );

            assert!(
                object
                    .get_field("expected")
                    .is_some()
            );

            assert!(
                object
                    .get_field("residuals")
                    .is_some()
            );
        }

        other => {
            panic!(
                "expected Object, got {:?}",
                other
            );
        }
    }
}

#[test]
fn chi_square_identical_distribution() {
    let result = run(
        r#"
        chi_square_gof(
            [25, 25, 25, 25],
            [25, 25, 25, 25]
        )
        "#
    );

    match result {
        Value::Object(object) => {
            let object = object.borrow();

            let statistic =
                match object
                    .get_field("statistic")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid statistic"),
                };

            let p =
                match object
                    .get_field("p_value")
                    .unwrap()
                {
                    Value::Float(v) => v,
                    _ => panic!("invalid p_value"),
                };

            assert_float_close(
                statistic,
                0.0,
            );

            assert_float_close(
                p,
                1.0,
            );
        }

        other => panic!(
            "expected Object, got {:?}",
            other
        ),
    }
}

#[test]
fn linear_regression() {
    let result =
        run(
            r#"
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

            linear_regression(X, y)
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
