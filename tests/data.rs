mod common;

use common::{
    run,
    run_result,
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
            import stats

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            stats.mean(
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

#[test]
fn dataframe_describe() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.describe()
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.columns(),
                vec![
                    "column",
                    "count",
                    "mean",
                    "std",
                    "min",
                    "median",
                    "max",
                ]
            );

            assert_eq!(
                df.nrows(),
                3
            );

            let mean =
                df.column("mean")
                    .unwrap();

            match mean.get(0) {
                Some(Value::Float(value)) => {
                    // age:
                    // (20 + 21 + 22 + 20 + 23 + 21) / 6
                    assert_float_close(
                        value,
                        127.0 / 6.0,
                    );
                }

                other => {
                    panic!(
                        "unexpected value: {:?}",
                        other
                    );
                }
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
fn dataframe_drop() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.drop([
                "reaction_time"
            ])
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.columns(),
                vec![
                    "condition",
                    "age",
                    "score",
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
fn dataframe_rename() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.rename({
                "reaction_time": "rt",
                "score": "result"
            })
            "#
        );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.columns(),
                vec![
                    "condition",
                    "age",
                    "rt",
                    "result",
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
fn dataframe_sort() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.sort(
                "score"
            )
            "#
        );

    match result {
        Value::DataFrame(df) => {
            let score =
                df.column("score")
                    .unwrap();

            assert_eq!(
                score.get(0),
                Some(Value::Float(74.5))
            );

            assert_eq!(
                score.get(5),
                Some(Value::Float(88.0))
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
fn dataframe_sort_descending() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.sort(
                "score",
                false
            )
            "#
        );

    match result {
        Value::DataFrame(df) => {
            let score =
                df.column("score")
                    .unwrap();

            assert_eq!(
                score.get(0),
                Some(Value::Float(88.0))
            );

            assert_eq!(
                score.get(5),
                Some(Value::Float(74.5))
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
fn grouped_dataframe_aggregate() {
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
            ).aggregate(
                "score",
                [
                    "count",
                    "mean",
                    "std",
                    "min",
                    "max"
                ]
            )
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
                    "score_count",
                    "score_mean",
                    "score_std",
                    "score_min",
                    "score_max",
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
fn scalar_and_short_circuit_still_works() {
    let result =
        run(
            "false and something"
        );

    assert_eq!(
        result,
        Value::Bool(false)
    );
}

#[test]
fn series_scalar_comparison() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.column("age") > 20
            "#
        );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.data(),
                &[
                    Value::Bool(false),
                    Value::Bool(true),
                    Value::Bool(true),
                    Value::Bool(false),
                    Value::Bool(true),
                    Value::Bool(true),
                ]
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
fn series_scalar_arithmetic() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/experiment.csv"
            )
            .column("age")
            + 10
            "#
        );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.get(0),
                Some(Value::Int(30))
            );

            assert_eq!(
                series.get(2),
                Some(Value::Int(32))
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
fn series_series_arithmetic() {
    let result =
        run(
            r#"
            import csv

            let age =
                csv.read(
                    "tests/data/experiment.csv"
                ).column("age")

            age + age
            "#
        );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.get(0),
                Some(Value::Int(40))
            );

            assert_eq!(
                series.get(2),
                Some(Value::Int(44))
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
fn dataframe_filter_boolean_series() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.filter(
                df.column("age") >= 21
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
fn dataframe_filter_compound_mask() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/experiment.csv"
                )

            df.filter(
                (df.column("age") >= 21)
                    and
                (df.column("score") > 80)
            )
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

#[test]
fn series_null_propagation() {
    let result =
        run(
            r#"
            import csv

            csv.read(
                "tests/data/missing.csv"
            )
            .column("score")
            + 10
            "#
        );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.get(0),
                Some(Value::Int(20))
            );

            assert_eq!(
                series.get(1),
                Some(Value::Null)
            );

            assert_eq!(
                series.get(2),
                Some(Value::Int(40))
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
fn series_mean_method() {
    let result = run(
        r#"
        import csv

        csv.read(
            "tests/data/experiment.csv"
        )
        .column("score")
        .mean()
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

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn series_median_method() {
    let result = run(
        r#"
        import csv

        csv.read(
            "tests/data/experiment.csv"
        )
        .column("score")
        .median()
        "#
    );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                (
                    79.0 + 81.5
                ) / 2.0,
            );
        }

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn series_quantile_method() {
    let result = run(
        r#"
        import csv

        csv.read(
            "tests/data/experiment.csv"
        )
        .column("score")
        .quantile(0.5)
        "#
    );

    match result {
        Value::Float(value) => {
            assert_float_close(
                value,
                (
                    79.0 + 81.5
                ) / 2.0,
            );
        }

        other => panic!(
            "expected Float, got {:?}",
            other
        ),
    }
}

#[test]
fn series_dropna() {
    let result = run(
        r#"
        import csv

        csv.read(
            "tests/data/missing.csv"
        )
        .column("score")
        .dropna()
        "#
    );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.len(),
                3
            );

            assert_eq!(
                series.get(0),
                Some(Value::Int(10))
            );

            assert_eq!(
                series.get(1),
                Some(Value::Int(30))
            );

            assert_eq!(
                series.get(2),
                Some(Value::Int(40))
            );
        }

        other => panic!(
            "expected Series, got {:?}",
            other
        ),
    }
}

#[test]
fn series_unique() {
    let result = run(
        r#"
        import csv

        let df =
            csv.read(
                "tests/data/experiment.csv"
            )

        df.column("condition")
            .unique()
        "#
    );

    match result {
        Value::Series(series) => {
            assert_eq!(
                series.data(),
                &[
                    Value::Str(
                        Rc::new("A".into())
                    ),
                    Value::Str(
                        Rc::new("B".into())
                    ),
                ]
            );
        }

        other => panic!(
            "expected Series, got {:?}",
            other
        ),
    }
}

#[test]
fn series_value_counts() {
    let result = run(
        r#"
        import csv

        let df =
            csv.read(
                "tests/data/experiment.csv"
            )

        df.column("condition")
            .value_counts()
        "#
    );

    match result {
        Value::DataFrame(df) => {
            assert_eq!(
                df.columns(),
                vec![
                    "value",
                    "count",
                ]
            );

            assert_eq!(
                df.nrows(),
                2
            );

            let count =
                df.column("count")
                    .unwrap();

            assert_eq!(
                count.get(0),
                Some(Value::Int(3))
            );

            assert_eq!(
                count.get(1),
                Some(Value::Int(3))
            );
        }

        other => panic!(
            "expected DataFrame, got {:?}",
            other
        ),
    }
}

#[test]
fn dataframe_crosstab() {
    let result =
        run(
            r#"
            import csv

            let df =
                csv.read(
                    "tests/data/categorical.csv"
                )

            df.crosstab(
                "condition",
                "outcome"
            )
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
                    "yes",
                    "no",
                ]
            );

            let yes =
                df.column("yes")
                    .unwrap();

            let no =
                df.column("no")
                    .unwrap();

            assert_eq!(
                yes.get(0),
                Some(Value::Int(2))
            );

            assert_eq!(
                no.get(0),
                Some(Value::Int(1))
            );

            assert_eq!(
                yes.get(1),
                Some(Value::Int(1))
            );

            assert_eq!(
                no.get(1),
                Some(Value::Int(2))
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
fn json_parse_object() {
    let result =
        run(
            r#"
            import json

            json.parse(
                "{\"name\":\"Alice\",\"age\":20}"
            )
            "#
        );

    match result {
        Value::Dict(dict) => {
            let dict =
                dict.borrow();

            assert_eq!(
                dict.get("name"),
                Some(
                    &Value::Str(
                        Rc::new(
                            "Alice".into()
                        )
                    )
                )
            );

            assert_eq!(
                dict.get("age"),
                Some(
                    &Value::Int(20)
                )
            );
        }

        other => panic!(
            "expected Dict, got {:?}",
            other
        ),
    }
}

#[test]
fn json_parse_nested() {
    let result =
        run(
            r#"
            import json

            let data =
                json.parse(
                    "{\"user\":{\"scores\":[10,20,30]}}"
                )

            data["user"]["scores"][1]
            "#
        );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn json_round_trip() {
    let result =
        run(
            r#"
            import json

            let value = {
                "name": "Alice",
                "age": 20,
                "scores": [80, 90]
            }

            let text =
                json.stringify(value)

            json.parse(text)
            "#
        );

    assert_eq!(
        result,
        Value::Dict(
            Rc::new(
                std::cell::RefCell::new(
                    std::collections::HashMap::from([
                        (
                            "name".into(),
                            Value::Str(
                                Rc::new("Alice".into())
                            )
                        ),
                        (
                            "age".into(),
                            Value::Int(20)
                        ),
                        (
                            "scores".into(),
                            Value::List(
                                Rc::new(
                                    std::cell::RefCell::new(
                                        vec![
                                            Value::Int(80),
                                            Value::Int(90),
                                        ]
                                    )
                                )
                            )
                        ),
                    ])
                )
            )
        )
    );
}

#[test]
fn fs_read_err() {
    let result =
        run(
            r#"
            import fs
            fs.read("does-not-exist.txt")
            "#
        );

    match result {
        Value::EnumValue(value) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Err"
            );
        }

        other => panic!(
            "expected Result, got {:?}",
            other
        ),
    }
}

#[test]
fn fs_read_propagates_error() {
    let result =
        run_result(
            r#"
            import fs

            fn_not_real = || {
                fs.read(
                    "does-not-exist.txt"
                )?
            }

            fn_not_real()
            "#
        );

    assert!(
        result.is_ok()
    );
}

#[test]
fn fs_exists() {
    let result =
        run(
            r#"
            import fs

            fs.exists(
                "Cargo.toml"
            )
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

