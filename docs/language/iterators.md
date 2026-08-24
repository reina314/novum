---
layout: page
title: Iterators
parent: Language Guide
---

# Iterators

Novum uses a lazy iterator abstraction for sequence pipelines.

## Automatic iterable conversion

Users normally do not need to call `.iter()` manually. Iterable values are automatically converted to an iterator when an iterator method is requested.

For example:

```novum
[1, 2, 3]
    .map(|x| x * 2)
    .filter(|x| x > 2)
    .collect()
```

The following classes of values can participate in the iterator protocol where implemented:

- `Iterator`
- `List`
- `Str`
- `Range`
- `Dict`
- `Set`, where available
- `Vector`

Matrix iteration is deliberately not treated as a generic iterable until its element semantics are fixed.

## Core methods

The iterator API includes:

```text
map(callback)
filter(callback)
enumerate()
zip(other)
take(n)
skip(n)
collect()
reduce(callback)
fold(initial, callback)
any(callback)
all(callback)
```

The exact callback contract for reduction methods follows the runtime implementation and should be checked against tests when writing reusable code.

## `map`

```novum
[1, 2, 3]
    .map(|x| x * 2)
    .collect()
```

## `filter`

```novum
[1, 2, 3, 4]
    .filter(|x| x % 2 == 0)
    .collect()
```

## `enumerate`

```novum
"abc"
    .enumerate()
    .collect()
```

## `zip`

Method form:

```novum
[1, 2, 3].zip([4, 5, 6])
```

Global functional form is also supported:

```novum
zip([1, 2, 3], [4, 5, 6])
```

The arguments to `zip` can be ordinary iterable values; explicit `.iter()` calls are not required.

## Laziness

Iterator adaptors such as `map`, `filter`, `take`, and `skip` produce iterators. `collect()` materializes the values into a list.

```novum
let result =
    (1..100)
        .filter(|x| x % 2 == 0)
        .take(10)
        .collect()
```

