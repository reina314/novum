---
layout: default
title: CSV
parent: Standard Library
nav_order: 3
---

# `csv`

Import with:

```novum
import csv
```

## `csv.read(path)`

Reads a CSV file and returns a `DataFrame`.

```novum
let df = csv.read("data/experiment.csv")
print(df)
```

The first row is interpreted as column headers. Each subsequent row must contain exactly the same number of fields as the header.

### Automatic value parsing

CSV fields are converted to Novum values using the following rules:

| CSV text | Novum value |
|---|---|
| empty text | `Null` |
| `NA`, `null`, `.` | `Null` |
| `true` / `false` | `Bool` |
| valid integer | `Int` |
| valid floating-point number | `Float` |
| anything else | `Str` |

For example:

```text
name,age,score,active
Alice,20,91.5,true
Bob,21,,false
```

can be consumed as a DataFrame whose values retain their natural Novum runtime types.

### Working with rows

Because DataFrames are iterable, you can process rows directly:

```novum
let young =
    csv.read("data/experiment.csv")
        .filter(|row| row["age"] < 25)
        .collect()
```

### Return and errors

`csv.read()` returns a `DataFrame` on success and reports file/CSV parsing failures as runtime errors.
