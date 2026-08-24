---
layout: page
title: Errors, Option, and Result
parent: Language Guide
---

# Errors, Option, and Result

Novum uses explicit runtime values for optional and fallible operations.

## `Option`

The language uses constructors such as:

```novum
Option.Some(value)
Option.None
```

A missing environment variable, absent dictionary/path metadata, or another optional operation can return `Option.None` rather than raising an error.

## `Result`

Fallible APIs return:

```novum
Result.Ok(value)
Result.Err(message)
```

Filesystem APIs use `Result` for operational failures.

## Propagation with `?`

```novum
let text = fs.read(path("data.txt"))?
```

This allows functions that consume fallible operations to avoid manually unpacking every `Result`.

## Control-flow errors

`return`, `break`, and `continue` are represented internally as `ControlFlow` values. The evaluator prevents invalid propagation, such as `break` escaping a loop or `return` escaping a module scope.

## Typical errors

Common runtime error categories include:

- `Name`: undefined or duplicate names.
- `Type`: invalid value type for an operation.
- `Arity`: wrong number of arguments.
- `Index`: invalid index or out-of-range access.
- `Import`: module resolution/loading errors.
- `Control`: invalid control-flow propagation.
- `Runtime`: general runtime failures.

