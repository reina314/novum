---
layout: page
title: Values and Types
parent: Language Guide
---

# Values and Types

Novum is dynamically evaluated, but the runtime has explicit type categories for diagnostics and standard-library APIs.

## Core values

The runtime currently includes categories such as:

| Type | Typical literal / origin |
|---|---|
| `Int` | `42` |
| `Float` | `3.14` |
| `Bool` | `true`, `false` |
| `Str` | `"hello"` |
| `List` | `[1, 2, 3]` |
| `Dict` | dictionary literals |
| `Tuple` | tuple expressions / pattern values |
| `Range` | `1..5`, `1..=5` |
| `Vector` | `list.vector()` / vector operations |
| `Matrix` | matrix literals and matrix operations |
| `Iterator` | iterator pipelines |
| `Option` values | `Option.Some(...)`, `Option.None` |
| `Result` values | `Result.Ok(...)`, `Result.Err(...)` |
| `Object` | struct/class instances |
| `Class` | struct/class runtime definitions |
| `Module` | imported module namespaces |
| `Function` / `Builtin` | callable values |
| `EnumValue` / `EnumConstructor` | enum values and constructors |
| `Path` | filesystem paths |
| `Unit` | no meaningful value |
| `Null` | null-like value used by some APIs |

The exact set is implementation-defined and may grow as the runtime evolves.

## `typeof`

The language provides a `typeof` builtin for inspecting runtime type information.

```novum
typeof(42)
typeof("hello")
```

## String conversion

`str(value)` converts a value to a string representation.

```novum
str(42)
str(3.14)
str(path("data/file.txt"))
```

## Numeric conversion

`int(text)` parses a string as an integer and `float(text)` parses a string as a floating-point value.

```novum
int("42")
float("3.14")
```

They are parsing operations rather than arbitrary coercions.

