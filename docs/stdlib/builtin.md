---
layout: default
title: Builtins
parent: Standard Library
nav_order: 1
---

# Builtins

Builtins are available directly; no `import` is required.

## Output and input

### `print(value, ...)`

Prints each argument on its own line and returns `Unit`.

```novum
print("hello")
print(1, 2, 3)
```

### `input()` / `input(prompt)`

Reads a line from standard input and returns it as a `Str`.

```novum
let name = input("Name: ")
```

## Inspection and conversion

### `typeof(value)`

Returns the runtime type name as `Str`.

```novum
typeof(42)        // "Int"
typeof([1, 2])    // "List"
```

### `str(value)`

Converts a value to its string representation. `Path` values are rendered as filesystem paths.

```novum
str(42)
str(path("data/file.csv"))
```

### `int(value)`

Converts an `Int` unchanged, converts a finite `Float` by numeric cast, or parses a `Str` as an integer.

```novum
int(3.8)
int("42")
```

### `float(value)`

Converts an `Int` to `Float`, leaves `Float` unchanged, or parses a numeric `Str`.

```novum
float(42)
float("3.14")
```

### `bool(value)`

Leaves `Bool` unchanged or parses the strings `"true"` and `"false"`.

```novum
bool("true")
bool("false")
```

### `is_null(value)`

Returns `Bool` indicating whether the value is `null`.

### `is_type(value, name)`

Returns `Bool` when `value` has the runtime type name given by the second argument.

```novum
is_type(42, "Int")
```

## Collections and data construction

### `len(value)`

Returns the length of a `List` or the number of Unicode characters in a `Str`.

```novum
len([1, 2, 3])
len("Novum")
```

### `zeros(count)`

Returns a `List` containing `count` integer zeros.

```novum
zeros(5)    // [0, 0, 0, 0, 0]
```

### `range(end)` / `range(start, end)`

Returns a `List` of integers with an exclusive end. `range(end)` produces `0` through `end - 1`; `range(start, end)` produces `start` through `end - 1`.

```novum
range(5)
range(2, 5)
```

Use ranges in `for` loops or iterator pipelines:

```novum
range(100)
    .filter(|x| x % 2 == 0)
    .take(5)
    .collect()
```

### `series(name, values)`

Creates a `Series` from a name and a `List`.

```novum
let score = series("score", [80, 92, 88])
```

### `dataframe(series_list)`

Creates a `DataFrame` from a `List` of `Series`.

```novum
let df = dataframe([
    series("age", [20, 21]),
    series("score", [80, 90])
])
```

All series must have compatible lengths for a valid DataFrame.

## Iterator helpers

### `iter(value)`

Explicitly converts an iterable value into an `Iterator`.

### `zip(left, right)`

Creates an iterator that pairs items from two iterable values.

### `enumerate(value)`

Creates an iterator of zero-based index/value pairs.

These helpers correspond to the iterator methods described in [Iterators and Pipelines](../tutorial/iterators.md).

## Paths

### `path(value)`

Converts a `Str` into a `Path` value or returns an existing `Path` unchanged.

```novum
let p = path("data/results.csv")
```

## Timing and randomness

### `sleep(milliseconds)`

Pauses execution for a non-negative integer number of milliseconds and returns `Unit`.

```novum
sleep(250)
```

### `random()`

Returns a pseudo-random `Float`.

### `randint(min, max)`

Returns a random `Int` in the inclusive range `[min, max]`.

```novum
let roll = randint(1, 6)
```

## Assertions and failure

### `assert(condition)`

Succeeds only when `condition` is `true`.

```novum
assert(score >= 0)
```

### `panic()` / `panic(value)`

Stops the current computation with the supplied message, or with the default `panic` message when no argument is given.
