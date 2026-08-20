mod common;

use common::{
    run,
    assert_float_close,
};
use novum::runtime::{Value};
use std::{rc::Rc};

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

#[test]
fn dataframe_filter() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.filter(
                |row| row.age >= 21
            )
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                4
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
fn dataframe_filter_string() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.filter(
                |row|
                    row.condition == "A"
            )
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                3
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
fn dataframe_group_by_count() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.group_by(
                "condition"
            ).count()
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                2
            );

            assert_eq!(
                df.ncols(),
                2
            );

            assert_eq!(
                df.columns(),
                vec![
                    "condition",
                    "count"
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
fn dataframe_group_by_mean() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.group_by(
                "condition"
            ).mean("score")
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                2
            );

            assert_eq!(
                df.columns(),
                vec![
                    "condition",
                    "score_mean"
                ]
            );

            let mean =
                df.column(
                    "score_mean"
                )
                .unwrap();

            // A:
            // (81.5 + 84.0 + 88.0) / 3
            //
            // B:
            // (76.5 + 79.0 + 74.5) / 3

            match mean.get(0) {
                Some(Value::Float(value)) => {
                    assert_float_close(
                        value,
                        84.5,
                    );
                }

                other => panic!(
                    "unexpected value: {:?}",
                    other
                ),
            }

            match mean.get(1) {
                Some(Value::Float(value)) => {
                    assert_float_close(
                        value,
                        (
                            76.5
                            + 79.0
                            + 74.5
                        ) / 3.0,
                    );
                }

                other => panic!(
                    "unexpected value: {:?}",
                    other
                ),
            }
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
fn dataframe_head() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/experiment.csv"
            ).head(2)
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.nrows(),
                2
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



