---
layout: default
title: A Small Data Workflow
parent: Tutorial
nav_order: 8
---

# A Small Data Workflow

This example combines imports, file input, DataFrames, row iteration, filtering, and numerical operations into one program.

## Read data

```novum
import csv
import linalg

let df = csv.read("data/experiment.csv")
```

`csv.read()` returns a `DataFrame`. Its rows can be consumed directly by iterator operations.

## Filter rows

```novum
let adults =
    df
        .filter(|row| row["age"] >= 20)
        .filter(|row| row["age"] <= 30)
        .collect()
```

This uses exactly the same iterator vocabulary as a list pipeline.

## Extract and transform a column

```novum
let scores =
    adults
        .map(|row| row["score"])
        .collect()
```

At this point `scores` is an ordinary list and can be passed to other builtins or converted into a vector.

```novum
let v = scores.vector()
print(v.norm())
```

## Numerical work

For matrix-style calculations, use `linalg`:

```novum
let X = linalg.matrix([
    [1, 10],
    [1, 12],
    [1, 15]
])

let y = linalg.vector([12, 15, 18])
let fit = linalg.linear_regression(X, y)

print(fit["coefficients"])
print(fit["r_squared"])
```

The same language therefore covers the full path from raw tabular input to numerical analysis without introducing a separate data-processing language.

> **Warning**
>
> The current `vm` line is still under development. Check the API reference before depending on behavior that is not demonstrated here.
