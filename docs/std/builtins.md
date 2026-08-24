---
layout: page
title: Builtins
parent: Standard Library
---

# Builtins

These operations are designed to be available without an explicit module import when installed by the current interpreter configuration.

## `print(value)`

Prints a value using its display representation.

```novum
print("hello")
print(42)
print("=" * 20)
```

## `input()`

Reads user input from standard input according to the current implementation.

```novum
let name = input()?
```

Check the host implementation for the exact return/error convention.

## `read(path)`

Legacy filesystem builtin for reading a UTF-8 text file. The current filesystem layer also exposes the richer `fs` module.

## `write(path, content)`

Legacy builtin for writing a text file.

## `append(path, content)`

Legacy builtin for appending text to a file.

## `str(value)`

Convert a value to a string representation.

```novum
str(42)
str(path("data.txt"))
```

## `int(text)`

Parse a string as an integer.

```novum
int("42")
```

## `float(text)`

Parse a string as a floating-point number.

```novum
float("3.14")
```

## `typeof(value)`

Return runtime type information.

## `iter(value)`

Explicitly convert an iterable value to an `Iterator`. This remains available, but most iterator methods now perform the conversion automatically.

## `zeros(n)`

Create a zero-filled list.

```novum
zeros(5)
```

produces:

```text
[0, 0, 0, 0, 0]
```

## `zip(a, b)`

Zip two iterable values without requiring explicit `.iter()` calls:

```novum
zip([1, 2, 3], [4, 5, 6])
```

## `args()`

Return command-line arguments excluding the executable name.

## `env(name)`

Legacy environment-variable builtin. The legacy API uses the older return convention; prefer `process.env(name)` for the newer module API.

## `cwd()`

Legacy current-working-directory builtin. Prefer `process.cwd()` when working with `Path` and `Result` values.

