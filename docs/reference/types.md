---
layout: default
title: Types and Values
parent: Reference
nav_order: 1
---

# Types and Values

Novum is dynamically evaluated. The runtime still distinguishes values by explicit type categories.

| Type | Typical construction |
|---|---|
| `Int` | `42` |
| `Float` | `3.14` |
| `Bool` | `true`, `false` |
| `Str` | `"hello"` |
| `List` | `[1, 2, 3]` |
| `Dict` | `{ "key": value }` |
| `Tuple` | `(x, y)` |
| `Range` | `1..5`, `1..=5` |
| `Vector` | `linalg.vector([...])` or list `.vector()` |
| `Matrix` | `linalg.matrix([[...]])` |
| `Series` | `series(name, list)` |
| `DataFrame` | `dataframe([series, ...])` |
| `Iterator` | `iter(value)` or iterator methods |
| `Option` | `Option.Some(...)`, `Option.None` |
| `Result` | `Result.Ok(...)`, `Result.Err(...)` |
| `Object` | struct/class construction |
| `Path` | `path(string)` |
| `Null` | `null` |
| `Unit` | operations with no useful value |

Use `typeof(value)` when runtime inspection is needed.
