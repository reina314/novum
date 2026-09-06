# Novum v0.17.4

> **Quick ideas. Quick experiments. Quick results.**

Novum is a small, practical programming language that combines compact syntax, expression-oriented programming, lazy iterators, data-oriented values, and a small standard library for research and scripting tasks.


## What is Novum?

Novum is designed for programs where the language should stay out of the way. Its syntax is intentionally compact, while the runtime provides useful building blocks for manipulating collections, working with files, invoking external processes, and performing numerical computation.

Novum is under active development, so the language and standard library may evolve over time.


## Example Usage
Code (`/samples/data_analysis_example.nv`):
```py
import csv
import stats
use stats

let df = csv.read("tests/data/experiment.csv")

print(df.describe())

let filtered =
    df.filter(|row|
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
        .filter(|row|
            row.condition == "A"
        )
        .column("score")

let c =
    filtered
        .filter(|row|
            row.condition == "C"
        )
        .column("score")

let result = welch(a, c)

for (k, v) in result {
    print(k + ": " + v)
}
```

Output:
```
DataFrame (3 rows x 7 columns)
column        | count | mean         | std        | min   | median | max
--------------+-------+--------------+------------+-------+--------+------
age           |   122 |  25.20491803 | 2.73314073 |  20.0 |   25.0 |  30.0
reaction_time |   122 | 495.05737705 | 31.4488432 | 440.0 |  494.0 | 558.0
score         |   122 |  79.46311475 | 6.40769425 |  66.5 |  80.25 |  90.0

DataFrame (4 rows x 4 columns)
condition | score_count | score_mean  | score_std
----------+-------------+-------------+-----------
A         |          31 | 85.16129032 | 2.99829701
B         |          31 | 76.74193548 | 2.30170577
C         |          30 | 84.81666667 | 2.24177936
D         |          30 | 71.03333333 | 2.39227685

method: Welch's t-test
df: 55.50894604
p_value: 0.61242184
statistic: 0.50949947
```