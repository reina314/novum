---
layout: default
title: Statistics
parent: Standard Library
nav_order: 8
---

# `stats`

The `stats` module provides descriptive statistics, effect sizes, hypothesis tests, analysis of variance, and post-hoc comparisons for `Series` and `DataFrame` values.

Import the module with:

```novum
import stats
```

Many descriptive statistics are available directly as methods on `Series` values. Statistical tests that require multiple inputs, a `DataFrame`, or explicit test parameters are exposed through the `stats` module.

---

## Descriptive statistics

The following functions operate on numeric `Series` values and are available as methods.

### `series.sum()`

Returns the sum of the numeric values.

```novum
let score = series("score", [10, 20, 30])

print(score.sum())
```

Returns:

```text
60
```

---

### `series.min()`

Returns the minimum value.

```novum
print(score.min())
```

---

### `series.max()`

Returns the maximum value.

```novum
print(score.max())
```

---

### `series.mean()`

Returns the arithmetic mean.

```novum
print(score.mean())
```

---

### `series.range()`

Returns the difference between the maximum and minimum values.

```novum
print(score.range())
```

For `[10, 20, 30]`, the result is `20`.

---

### `series.median()`

Returns the median.

```novum
let score = series("score", [10, 20, 30, 40])

print(score.median())
```

The implementation uses linear interpolation between adjacent ordered values when necessary.

---

### `series.quantile(q)`

Returns the quantile at `q`, where `q` must be in the range `[0, 1]`.

```novum
let score = series("score", [10, 20, 30, 40, 50])

print(score.quantile(0.25))
print(score.quantile(0.5))
print(score.quantile(0.75))
```

`0.5` is equivalent to the median.

---

### `series.variance()`

Returns the sample variance.

```novum
print(score.variance())
```

At least two observations are required. When the variance is undefined, the result is `null`.

---

### `series.std()`

Returns the sample standard deviation.

```novum
print(score.std())
```

This is the square root of the sample variance.

---

### `series.skewness()`

Returns the sample skewness.

```novum
print(score.skewness())
```

At least three observations are required. Otherwise, the result is `null`.

---

### `series.kurtosis()`

Returns the sample excess kurtosis.

```novum
print(score.kurtosis())
```

At least four observations are required. Otherwise, the result is `null`.

---

### `series.covariance(other)`

Returns the sample covariance between two numeric series.

```novum
let x = series("x", [1, 2, 3, 4])
let y = series("y", [2, 4, 6, 8])

print(x.covariance(y))
```

The two series must have the same length and contain at least two observations.

---

### `series.correlation(other)`

Returns the Pearson correlation coefficient between two numeric series.

```novum
let x = series("x", [1, 2, 3, 4])
let y = series("y", [2, 4, 6, 8])

print(x.correlation(y))
```

A correlation cannot be determined when either series has zero variance.

---

## DataFrame column access

A `DataFrame` column can be accessed explicitly with `column()`:

```novum
let score = df.column("score")
```

Columns can also be accessed directly by name:

```novum
let score = df.score
let age = df.age
```

These forms return the same `Series`.

This is useful when combining the DataFrame API with statistical functions:

```novum
print(df.score.mean())
print(df.reaction_time.std())
```

DataFrame properties such as `nrows`, `ncols`, and `columns` remain reserved properties.

---

## Describing a DataFrame

### `stats.describe(df)`

Returns a `DataFrame` containing descriptive statistics for the numeric columns of `df`.

```novum
let summary = stats.describe(df)

print(summary)
```

The result contains one row per numeric input column.

| Column   | Meaning                        |
| -------- | ------------------------------ |
| `column` | Original column name           |
| `count`  | Number of numeric observations |
| `mean`   | Arithmetic mean                |
| `std`    | Sample standard deviation      |
| `min`    | Minimum                        |
| `median` | Median                         |
| `max`    | Maximum                        |

Non-numeric columns are not included.

---

# Hypothesis tests

Statistical tests return dictionaries containing the main test statistics and, where implemented, estimates, confidence intervals, and effect sizes.

For example:

```novum
let result = stats.ttest(sample, 0)

print(result.statistic)
print(result.p_value)
print(result.effect_size)
```

The result dictionaries are ordinary Novum dictionaries and can therefore be accessed with field syntax.

---

## One-sample t-test

### `stats.ttest(series, mu0 [, confidence])`

Performs a two-sided one-sample t-test of

```text
H0: mean = mu0
```

Arguments:

| Argument     | Type     | Description                                   |
| ------------ | -------- | --------------------------------------------- |
| `series`     | `Series` | Numeric sample                                |
| `mu0`        | numeric  | Mean under the null hypothesis                |
| `confidence` | numeric  | Optional confidence level; defaults to `0.95` |

Example:

```novum
let score = series("score", [10, 12, 11, 13, 14])

let result = stats.ttest(score, 10)

print(result)
```

The result contains:

| Field                 | Meaning                                             |
| --------------------- | --------------------------------------------------- |
| `statistic`           | t statistic                                         |
| `p_value`             | Two-sided p-value                                   |
| `df`                  | Degrees of freedom                                  |
| `estimate`            | Sample mean minus `mu0`                             |
| `effect_size`         | Cohen's d                                           |
| `effect_size_name`    | `"Cohen's d"`                                       |
| `effect_size_ci`      | Approximate confidence interval for the effect size |
| `confidence_interval` | Confidence interval for the mean difference         |
| `method`              | Test name                                           |

The sample must contain at least two finite observations and must have non-zero sample standard deviation.

---

## Paired t-test

### `stats.paired_ttest(first, second [, confidence])`

Performs a two-sided paired t-test.

Each observation in `first` is paired with the observation at the same position in `second`.

```novum
let before = series("before", [10, 12, 11, 13, 14])
let after = series("after", [12, 13, 13, 15, 16])

let result = stats.paired_ttest(before, after)

print(result)
```

Pairs containing `null` in either series are omitted.

The result contains the same general fields as the one-sample t-test, but the effect size is:

```text
Cohen's dz
```

The relevant fields are:

| Field                 | Meaning                                            |
| --------------------- | -------------------------------------------------- |
| `statistic`           | Paired t statistic                                 |
| `p_value`             | Two-sided p-value                                  |
| `df`                  | Degrees of freedom                                 |
| `estimate`            | Mean paired difference                             |
| `effect_size`         | Cohen's dz                                         |
| `effect_size_name`    | `"Cohen's dz"`                                     |
| `effect_size_ci`      | Approximate confidence interval for Cohen's dz     |
| `confidence_interval` | Confidence interval for the mean paired difference |
| `method`              | `"Paired t-test"`                                  |

At least two complete pairs are required.

---

## Welch's t-test

### `series.welch(other [, confidence])`

Performs Welch's two-sample t-test without assuming equal population variances.

```novum
let control = series("control", [12, 14, 15, 13, 14])
let treatment = series("treatment", [17, 18, 16, 19, 20])

let result = control.welch(treatment)

print(result)
```

The optional confidence level defaults to `0.95`.

The result contains:

| Field                 | Meaning                                       |
| --------------------- | --------------------------------------------- |
| `statistic`           | Welch t statistic                             |
| `p_value`             | Two-sided p-value                             |
| `df`                  | Welch-Satterthwaite degrees of freedom        |
| `estimate`            | Difference between sample means               |
| `effect_size`         | Cohen's d                                     |
| `effect_size_name`    | `"Cohen's d"`                                 |
| `effect_size_ci`      | Approximate confidence interval for Cohen's d |
| `hedges_g`            | Hedges' g                                     |
| `hedges_g_ci`         | Approximate confidence interval for Hedges' g |
| `confidence_interval` | Confidence interval for the mean difference   |
| `method`              | `"Welch's t-test"`                            |

Each group must contain at least two finite observations.

---

# Effect sizes

Effect sizes can be calculated independently of a hypothesis test.

## `stats.cohens_d(first, second)`

Returns Cohen's d for two independent samples.

```novum
let control = series("control", [10, 11, 12, 13])
let treatment = series("treatment", [15, 16, 17, 18])

let d = stats.cohens_d(control, treatment)

print(d)
```

The calculation uses the pooled sample standard deviation.

At least two observations are required in each group.

---

## `stats.hedges_g(first, second)`

Returns Hedges' g for two independent samples.

```novum
let control = series("control", [10, 11, 12, 13])
let treatment = series("treatment", [15, 16, 17, 18])

let g = stats.hedges_g(control, treatment)

print(g)
```

Hedges' g applies a small-sample correction to Cohen's d.

---

# Non-parametric tests

## Mann-Whitney U test

### `stats.mann_whitney(first, second)`

Performs a two-sided Mann-Whitney U test using a normal approximation with tie correction and continuity correction.

```novum
let control = series("control", [10, 12, 13, 15])
let treatment = series("treatment", [16, 17, 19, 20])

let result = stats.mann_whitney(control, treatment)

print(result)
```

The result contains:

| Field       | Meaning                          |
| ----------- | -------------------------------- |
| `statistic` | Mann-Whitney U statistic         |
| `p_value`   | Two-sided p-value                |
| `z`         | Normal-approximation z statistic |
| `method`    | `"Mann-Whitney U test"`          |

Ties are handled using average ranks.

---

## Wilcoxon signed-rank test

### `stats.wilcoxon(first, second)`

Performs a two-sided Wilcoxon signed-rank test for paired observations.

```novum
let before = series("before", [10, 12, 11, 13, 14])
let after = series("after", [12, 13, 13, 15, 16])

let result = stats.wilcoxon(before, after)

print(result)
```

Zero differences are omitted before ranking.

The result contains:

| Field       | Meaning                          |
| ----------- | -------------------------------- |
| `statistic` | Wilcoxon signed-rank statistic   |
| `p_value`   | Two-sided p-value                |
| `z`         | Normal-approximation z statistic |
| `n`         | Number of non-zero differences   |
| `method`    | `"Wilcoxon signed-rank test"`    |

The implementation uses average ranks for ties and a normal approximation with continuity correction.

---

# One-way ANOVA

## `stats.anova(df, response, factor)`

Performs a one-way analysis of variance.

```novum
let result = stats.anova(
    df,
    "score",
    "condition"
)

print(result)
```

The response column must be numeric. The factor column may contain integer, floating-point, Boolean, or string categories. `null` factor values and `null` response values are omitted from the analysis.

The result contains:

| Field                          | Meaning                          |
| ------------------------------ | -------------------------------- |
| `statistic`                    | F statistic                      |
| `p_value`                      | ANOVA p-value                    |
| `df_between`                   | Between-group degrees of freedom |
| `df_within`                    | Within-group degrees of freedom  |
| `effect_size`                  | Eta squared                      |
| `effect_size_name`             | `"Eta squared"`                  |
| `alternative_effect_size`      | Omega squared                    |
| `alternative_effect_size_name` | `"Omega squared"`                |
| `confidence_interval`          | Currently `null`                 |
| `effect_size_ci`               | Currently `null`                 |
| `method`                       | `"One-way ANOVA"`                |

Eta squared is the proportion of total variance explained by the factor.

Omega squared is provided as a less-biased alternative effect-size estimate.

---

# Tukey HSD

ANOVA determines whether there is evidence that at least one group differs from another. It does not identify which pairs differ.

Use Tukey HSD for pairwise comparisons after ANOVA.

## `stats.tukey(df, response, factor [, confidence])`

Performs Tukey's HSD test for all pairwise group comparisons.

```novum
let anova_result =
    stats.anova(df, "score", "condition")

let post_hoc =
    stats.tukey(df, "score", "condition")

print(anova_result)
print(post_hoc)
```

The optional confidence level defaults to `0.95`.

The current implementation supports confidence levels:

* `0.95`
* `0.99`

At least two groups are required, and at most ten groups are currently supported.

Each group must contain at least two observations.

The result is a `DataFrame` with one row per pairwise comparison.

| Column        | Meaning                                                     |
| ------------- | ----------------------------------------------------------- |
| `group1`      | First group                                                 |
| `group2`      | Second group                                                |
| `mean_diff`   | Mean of `group1` minus mean of `group2`                     |
| `q`           | Tukey q statistic                                           |
| `p_value`     | Tukey-adjusted p-value                                      |
| `ci_lower`    | Lower confidence bound for the mean difference              |
| `ci_upper`    | Upper confidence bound for the mean difference              |
| `significant` | Whether the comparison is significant at the selected alpha |

Because the result is a `DataFrame`, it can be processed using the ordinary DataFrame API:

```novum
let significant =
    stats.tukey(df, "score", "condition")
        .filter(|row| row.p_value < 0.05)

print(significant)
```

The order of factor groups follows their first appearance in the input data.

---

# Post-hoc API

## `stats.post_hoc(df, response, factor [, method [, confidence]])`

`post_hoc()` provides a general entry point for post-hoc procedures.

The default method is Tukey HSD.

```novum
let result =
    stats.post_hoc(
        df,
        "score",
        "condition"
    )

print(result)
```

This is equivalent to:

```novum
let result =
    stats.tukey(
        df,
        "score",
        "condition"
    )
```

The method can be specified explicitly:

```novum
let result =
    stats.post_hoc(
        df,
        "score",
        "condition",
        "tukey"
    )
```

The alias `"tukey_hsd"` is also accepted.

To specify a different confidence level, provide it as the fifth argument:

```novum
let result =
    stats.post_hoc(
        df,
        "score",
        "condition",
        "tukey",
        0.99
    )
```

Currently supported methods:

| Method        | Description         |
| ------------- | ------------------- |
| `"tukey"`     | Tukey HSD           |
| `"tukey_hsd"` | Alias for Tukey HSD |

Unknown post-hoc methods produce an error.

The `post_hoc()` interface is intended as the common entry point for future post-hoc procedures.

---

# Chi-square test

## `stats.chi_square(df, first, second)`

Performs a chi-square test of independence between two categorical columns.

```novum
let result =
    stats.chi_square(
        df,
        "condition",
        "outcome"
    )

print(result)
```

The two columns may contain integer, floating-point, Boolean, or string categories.

`null` observations are omitted.

The result contains:

| Field                 | Meaning                             |
| --------------------- | ----------------------------------- |
| `statistic`           | Chi-square statistic                |
| `p_value`             | p-value                             |
| `df`                  | Degrees of freedom                  |
| `effect_size`         | Cramér's V                          |
| `effect_size_name`    | `"Cramer's V"`                      |
| `confidence_interval` | Currently `null`                    |
| `effect_size_ci`      | Currently `null`                    |
| `method`              | `"Chi-square test of independence"` |

At least two categories are required in each variable.

---

# Typical analysis workflow

For a typical experimental dataset, the statistical workflow can be written directly as a sequence of DataFrame operations and statistical functions.

```novum
import csv
import stats

let df = csv.read("tests/data/experiment.csv")

print(stats.describe(df))

let filtered =
    df.filter(
        |row|
            row.age >= 20
                and
            row.age <= 30
    )

let grouped =
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

print(grouped)

let anova_result =
    stats.anova(
        filtered,
        "score",
        "condition"
    )

print(anova_result)

let post_hoc =
    stats.post_hoc(
        filtered,
        "score",
        "condition",
        "tukey"
    )

print(post_hoc)
```

This separates the analysis into three stages:

```text
DataFrame operations
    ↓
descriptive / grouped summaries
    ↓
omnibus statistical test
    ↓
post-hoc pairwise comparisons
```

For a one-way experimental design, ANOVA can therefore be followed by Tukey HSD to identify which specific group pairs differ.

---

# Missing and invalid values

The statistics API distinguishes between missing values and invalid numeric values.

`null` observations are generally omitted where the operation can do so naturally, such as paired tests and ANOVA.

Non-finite floating-point values such as `NaN` and infinities are rejected by inferential procedures that require finite numeric data.

When there are not enough observations to calculate a descriptive statistic, functions such as `median()`, `variance()`, `std()`, `skewness()`, and `kurtosis()` may return `null`.

---

# Summary

The main `stats` APIs are:

| Function                    | Input                 | Result          |
| --------------------------- | --------------------- | --------------- |
| `series.sum()`              | `Series`              | Number          |
| `series.min()`              | `Series`              | Number          |
| `series.max()`              | `Series`              | Number          |
| `series.mean()`             | `Series`              | Number          |
| `series.range()`            | `Series`              | Number          |
| `series.median()`           | `Series`              | Number          |
| `series.quantile(q)`        | `Series`, `q`         | Number          |
| `series.variance()`         | `Series`              | Number          |
| `series.std()`              | `Series`              | Number          |
| `series.skewness()`         | `Series`              | Number / `null` |
| `series.kurtosis()`         | `Series`              | Number / `null` |
| `series.covariance(other)`  | Two `Series`          | Number          |
| `series.correlation(other)` | Two `Series`          | Number          |
| `stats.describe(df)`        | `DataFrame`           | `DataFrame`     |
| `stats.ttest(...)`          | `Series`, mean        | Dictionary      |
| `stats.paired_ttest(...)`   | Two `Series`          | Dictionary      |
| `series.welch(...)`         | Two `Series`          | Dictionary      |
| `stats.cohens_d(...)`       | Two `Series`          | Number          |
| `stats.hedges_g(...)`       | Two `Series`          | Number          |
| `stats.mann_whitney(...)`   | Two `Series`          | Dictionary      |
| `stats.wilcoxon(...)`       | Two `Series`          | Dictionary      |
| `stats.anova(...)`          | `DataFrame` + columns | Dictionary      |
| `stats.tukey(...)`          | `DataFrame` + columns | `DataFrame`     |
| `stats.post_hoc(...)`       | `DataFrame` + columns | `DataFrame`     |
| `stats.chi_square(...)`     | `DataFrame` + columns | Dictionary      |

The statistical API is designed to work naturally with Novum's `Series` and `DataFrame` operations, allowing data manipulation and statistical analysis to remain part of the same workflow.
