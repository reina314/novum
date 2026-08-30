---
layout: default
title: Iterators and Pipelines
parent: Tutorial
nav_order: 7
---

# Iterators and Pipelines

Iteration is one of Novum's core strengths. The language treats many common values as iterable and lets you build lazy transformations with ordinary method calls.

## Iterable values

The current iterator system accepts:

- `Iterator`
- `List`
- `Str`
- `Range`
- `Dict`
- `Vector`
- `Series`
- `DataFrame`

That means you normally do **not** need to call `iter()` before using an iterator method.

```novum
[1, 2, 3].map(|x| x * 2).collect()
```

## `map`

Transform each element:

```novum
let squares =
    [1, 2, 3, 4]
        .map(|x| x * x)
        .collect()
```

## `filter`

Keep elements for which the predicate returns `true`:

```novum
let even =
    (1..10)
        .filter(|x| x % 2 == 0)
        .collect()
```

## `take` and `skip`

```novum
let first_three =
    (0..100)
        .take(3)
        .collect()

let after_five =
    (0..10)
        .skip(5)
        .collect()
```

These operations stay lazy; only the requested part of the source needs to be consumed.

## `enumerate`

Attach an index to each item:

```novum
let indexed =
    ["a", "b", "c"]
        .enumerate()
        .collect()
```

Each element is represented as a tuple containing the zero-based index and the source value.

## `zip`

Combine two iterables element-by-element:

```novum
let pairs =
    [1, 2, 3]
        .zip([10, 20, 30])
        .collect()
```

There is also a builtin functional form:

```novum
let pairs = zip([1, 2, 3], [10, 20, 30]).collect()
```

## `reduce` and `fold`

Use a reduction when you want a single accumulated value:

```novum
let total =
    [1, 2, 3, 4]
        .fold(0, |acc, x| acc + x)
```

`fold` takes an explicit initial accumulator. `reduce` combines elements without a separate initial value.

## `any` and `all`

```novum
let has_large =
    [2, 4, 8]
        .any(|x| x > 5)

let all_even =
    [2, 4, 8]
        .all(|x| x % 2 == 0)
```

## Materialization with `collect`

Adapter methods such as `map`, `filter`, `take`, `skip`, `zip`, and `enumerate` return iterators. `collect()` turns the current iterator into a `List`.

```novum
let result =
    (1..100)
        .filter(|x| x % 3 == 0)
        .map(|x| x * 10)
        .take(5)
        .collect()
```

## Explicit `iter()`

The builtin `iter(value)` is available when you want an iterator value explicitly:

```novum
let it = iter([1, 2, 3])
print(it)
```

In ordinary application code, implicit conversion makes this less common.

## Why this style works well

A pipeline separates *what to do* from *how to loop*:

```novum
let top_scores =
    rows
        .filter(|row| row["valid"] == true)
        .map(|row| row["score"])
        .take(10)
        .collect()
```

This same pattern can be applied to lists, strings, vectors, series, ranges, and DataFrames.
