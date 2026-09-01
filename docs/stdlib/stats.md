---

layout: default
title: Statistics
parent: Standard Library
nav_order: 8
------------

# `stats`

Import with:

```
import stats
```

The `stats` module provides descriptive statistics and statistical tests for `Series` and `DataFrame` values.

## Descriptive statistics

The following operations are available as methods on `Series` values.

### `series.sum()`

Returns the sum of the numeric values.

```
let x = series("score", [10, 20, 30])
print(x.sum())
```

### `series.min()`

Returns the minimum value.

### `series.max()`

Returns the maximum value.

### `series.mean()`

Returns the arithmetic mean.

### `series.median()`

Returns the median.

### `series.quantile(q)`

Returns the quantile at `q`.

```
x.quantile(0.5)
```

### `series.variance()`

Returns the sample variance.

### `series.std()`

Returns the sample standard deviation.

### `series.correlation(other)`

Returns the correlation between two numeric series.

```
let x = series("x", [1, 2, 3, 4])
let y = series("y", [2, 4, 6, 8])

print(x.correlation(y))
```

## Describing a DataFrame

### `stats.describe(df)`

Returns a `DataFrame` containing descriptive statistics for the numeric columns of `df`.

```
let summary = stats.describe(df)
print(summary)
```

The result contains the following columns.

| Column   | Meaning                   |
| -------- | ------------------------- |
| `column` | Original column name      |
| `count`  | Number of observations    |
| `mean`   | Arithmetic mean           |
| `std`    | Sample standard deviation |
| `min`    | Minimum                   |
| `median` | Median                    |
| `max`    | Maximum                   |

## Statistical tests

### `stats.ttest(...)`

Performs a t-test using the supplied data and parameters.

### `series.welch(other)`

Performs Welch's t-test between two numeric series.

```
let a = series("control", [12, 14, 15, 13])
let b = series("treatment", [17, 18, 16, 19])

let result = a.welch(b)
```

The returned value is a dictionary containing the test statistic and corresponding probability information.

> **Note**
>
> The statistical API is based on the current implementation and may evolve as additional tests are added.