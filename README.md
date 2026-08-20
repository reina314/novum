# Novum v0.5.0

### For quick ideas, quick experiments, and quick results.

Novum is a lightweight programming language for small tasks, quick experiments, and everyday research work. Rather than aiming to be a large, feature-heavy language, Novum focuses on being simple, flexible, and easy to pick up. Its concise syntax and built-in statistical tools make it a convenient companion when you need to test an idea, manipulate some data, or run a quick analysis. This is a structural rewrite of the original Parvum interpreter.

## Sample Code
```py
import csv
import stats

let df = csv.read("tests/data/experiment.csv")

print(df)

let filtered =
    df.filter(|row|
        row.age >= 21
    )

let grouped =
    filtered.group_by(
        "condition"
    )

let summary =
    grouped.mean("score")

print(summary)
```