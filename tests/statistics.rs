mod common;

use common::{
    run,
    assert_float_close,
};
use novum::runtime::{Value};
use std::rc::Rc;

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

#[test]
fn csv_read() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/experiment.csv"
            ).nrows
            "#
        );

    assert_eq!(
        result,
        Value::Int(6)
    );
}

#[test]
fn dataframe_ncols() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/experiment.csv"
            ).ncols
            "#
        );

    assert_eq!(
        result,
        Value::Int(4)
    );
}

#[test]
fn dataframe_columns() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/experiment.csv"
            ).columns
            "#
        );

    match result {
        Value::List(values) => {
            let values =
                values.borrow();

            assert_eq!(
                values.len(),
                4
            );

            assert_eq!(
                values[0],
                Value::Str(
                    Rc::new(
                        "condition".into()
                    )
                )
            );

            assert_eq!(
                values[1],
                Value::Str(
                    Rc::new(
                        "age".into()
                    )
                )
            );
        }

        other => {
            panic!(
                "expected List, got {:?}",
                other
            );
        }
    }
}

#[test]
fn dataframe_column() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.column("score")
            "#
        );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.len(),
                6
            );

            assert_eq!(
                series.name(),
                "score"
            );

            assert_eq!(
                series.get(0),
                Some(Value::Float(81.5))
            );
        }

        other => {
            panic!(
                "expected Series, got {:?}",
                other
            );
        }
    }
}

#[test]
fn dataframe_series_statistics() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            mean(
                df.column("score")
            )
            "#
        );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                (
                    81.5
                    + 84.0
                    + 76.5
                    + 79.0
                    + 88.0
                    + 74.5
                ) / 6.0,
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
fn dataframe_select() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.select([
                "age",
                "score"
            ])
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                6
            );

            assert_eq!(
                df.ncols(),
                2
            );

            assert_eq!(
                df.columns(),
                vec![
                    "age",
                    "score"
                ]
            );
        }

        other => {
            panic!(
                "expected DataFrame, got {:?}",
                other
            );
        }
    }
}

#[test]
fn dataframe_to_matrix() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.select([
                "age",
                "score"
            ]).to_matrix()
            "#
        );

    match result {
        Value::Matrix(matrix) => {
            let matrix =
                matrix.borrow();

            assert_eq!(
                matrix.shape(),
                (6, 2)
            );

            assert_float_close(
                matrix.get(0, 0).unwrap(),
                20.0,
            );

            assert_float_close(
                matrix.get(0, 1).unwrap(),
                81.5,
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
fn dataframe_to_regression() {
    let result =
        run(
            r#"
            import csv
            import linalg

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            let X =
                df.select([
                    "age",
                    "reaction_time"
                ]).to_matrix()

            let y =
                df.column(
                    "score"
                ).to_matrix()

            linalg.linear_regression(
                X,
                y
            )
            "#
        );

    match result {
        Value::Object(object) => {
            let object =
                object.borrow();

            assert!(
                object
                    .get_field(
                        "r_squared"
                    )
                    .is_some()
            );

            assert!(
                object
                    .get_field(
                        "coefficients"
                    )
                    .is_some()
            );
        }

        other => {
            panic!(
                "expected regression Object, got {:?}",
                other
            );
        }
    }
}

