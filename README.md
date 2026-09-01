# Novum v0.17.2

> **Quick ideas. Quick experiments. Quick results.**

Novum is a small, practical programming language that combines compact syntax, expression-oriented programming, lazy iterators, data-oriented values, and a small standard library for research and scripting tasks.


## What is Novum?

Novum is designed for programs where the language should stay out of the way. Its syntax is intentionally compact, while the runtime provides useful building blocks for manipulating collections, working with files, invoking external processes, and performing numerical computation.

Novum is under active development, so the language and standard library may evolve over time. These pages document the behavior of the current implementation.


## Example Usage
Code (`/samples/data_analysis_example.nv`):
```py
import csv
import stats

let df = csv.read(
            "tests/data/experiment.csv"
        )

print(df.describe())

let filtered =
    df.filter(
        |row|
            row.age >= 20
                and 
            row.age <= 30
    )

let summary =
    filtered
        .group_by("condition")
        .aggregate(
            "score",
            [
                "count",
                "mean",
                "std"
            ]
        )

print(summary)

let a =
    filtered
        .filter(
            |row|
                row.condition == "A"
        )
        .column("score")

let b =
    filtered
        .filter(
            |row|
                row.condition == "B"
        )
        .column("score")

let result = stats.welch_t(a, b)

print(result)
```

Output:
```
DataFrame (3 rows x 7 columns)
column        | count | mean         | std         | min  | median | max
--------------+-------+--------------+-------------+------+--------+----
age           |     6 |  21.16666667 |  1.16904519 |   20 |     21 |  23
reaction_time |     6 | 484.16666667 | 28.35783255 |  450 |  482.5 | 520
score         |     6 |  80.58333333 |  4.97409958 | 74.5 |  80.25 |  88

DataFrame (2 rows x 4 columns)
condition | score_count | score_mean  | score_std
----------+-------------+-------------+-----------
A         |           3 |        84.5 | 3.27871926
B         |           3 | 76.66666667 | 2.25462488

{df: 3.54582065, mean_x: 84.5, mean_y: 76.66666667, p_value: 0.03261546, statistic: 3.40973838, test: Welch's t-test}
```