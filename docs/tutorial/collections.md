---
layout: default
title: Collections and Data Values
parent: Tutorial
nav_order: 4
---

# Collections and Data Values

Novum provides lists and dictionaries as general-purpose containers, plus specialized data types for numerical and tabular work.

## Lists

Create a list with square brackets:

```novum
let xs = [1, 2, 3]
```

Lists are mutable and support:

```novum
xs.push(4)
let last = xs.pop()
xs.remove(0)
print(xs.len())
```

A list can be repeated with multiplication:

```novum
let zeros = [0] * 5
```

## Dictionaries

Dictionary literals use braces with string keys:

```novum
let person = {
    "name": "Ada",
    "age": 36
}

print(person["name"])
```

Dictionaries are directly iterable. Each iteration yields a key/value tuple.

## Strings

Strings support useful methods:

```novum
let text = "  Novum  "

print(text.trim())
print(text.to_upper())
print(text.to_lower())
print(text.contains("vum"))
print(text.len())
```

Strings are also directly iterable, yielding one-character strings.

## Vectors

Convert a numeric list to a vector:

```novum
let v = [3, 4].vector()
print(v.norm())
```

Vector values participate in numeric operations and can be consumed by iterators.

## Matrices

Matrices are represented by nested lists:

```novum
import linalg

let A = linalg.matrix([
    [1, 2],
    [3, 4]
])

print(A.shape())
```

Matrix multiplication uses `@`:

```novum
let B = linalg.matrix([
    [5, 6],
    [7, 8]
])

let C = A @ B
```

## Series and DataFrames

The builtins `series()` and `dataframe()` create tabular values:

```novum
let age = series("age", [20, 21, 23])
let score = series("score", [80.0, 91.5, 87.0])
let df = dataframe([age, score])
```

A DataFrame row is exposed as a dictionary-like object when you iterate over the frame:

```novum
for row in df {
    print(row["score"])
}
```

This row-oriented behavior makes DataFrames fit naturally into iterator pipelines.
