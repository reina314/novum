---
layout: default
title: Tutorial
nav_order: 3
has_children: true
permalink: /tutorial/
---

# Tutorial

The Novum tutorial is organized around the way you actually write programs: bind values, define functions, control execution, build collections, model data, and then use iterator pipelines to compose transformations.

## Recommended path

1. [Variables and expressions](variables.md)
2. [Functions and lambdas](functions.md)
3. [Control flow](control-flow.md)
4. [Collections and indexing](collections.md)
5. [Structs and classes](structs-classes.md)
6. [Modules, enums, and pattern matching](modules-patterns.md)
7. [Iterators and pipelines](iterators.md)
8. [Putting it together](data-workflow.md)

## The central idea

Novum favors small expressions that can be combined. The language becomes particularly useful when ordinary values and data-oriented objects are fed into the same iterator pipeline:

```novum
let answer =
    [3, 8, 2, 9, 4, 10]
        .filter(|x| x >= 4)
        .map(|x| x * x)
        .take(3)
        .collect()

print(answer)
```

The rest of the tutorial explains each part of this style in isolation before combining them.
