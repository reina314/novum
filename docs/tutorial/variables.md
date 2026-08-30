---
layout: default
title: Variables and Expressions
parent: Tutorial
nav_order: 1
---

# Variables and Expressions

Novum is dynamically evaluated, but values have explicit runtime categories such as `Int`, `Float`, `Bool`, `Str`, `List`, `Dict`, `Vector`, `Matrix`, `Iterator`, `Object`, `Path`, `Null`, and `Unit`.

## Literals

The basic literals are straightforward:

```novum
42
3.14
true
false
"hello"
[1, 2, 3]
```

Integer and floating-point literals may contain underscores for readability:

```novum
let million = 1_000_000
let ratio = 3.141_592
```

Strings may use single or double quotes. `//` starts a line comment.

## Binding with `let`

Use `let` for an explicit binding:

```novum
let width = 12
let height = 8
let area = width * height
```

Top-level bindings can be exported with `pub`:

```novum
pub let version = "0.1"
```

A binding pattern can destructure a value:

```novum
let (x, y) = (10, 20)
```

## Assignment

Novum also supports ordinary assignment:

```novum
x = 10
x = 20
```

Assignment targets can be names, indexed elements, dictionary entries, or object fields:

```novum
xs[0] = 99
data["score"] = 100
point.x = 5
```

Compound assignment uses the usual operators:

```novum
x += 1
x *= 2
point.x -= 1
```

## Expressions

Blocks are expressions. The value of a block is the value of its final expression:

```novum
let area = {
    let width = 10
    let height = 4
    width * height
}
```

This makes it possible to introduce local names without creating a separate statement-only construct.

## Access and indexing

Field access:

```novum
object.name
```

Indexing:

```novum
items[0]
items[1..4]
dict["key"]
```

Tuple elements can be accessed by numeric field syntax:

```novum
let point = (3, 4)
print(point.0)
```

## Type inspection and conversion

```novum
print(typeof(42))
print(str(42))
print(int("123"))
print(float("3.14"))
print(bool("true"))
```

`int`, `float`, and `bool` are explicit conversions/parsers; unsupported input results in an error rather than silent coercion.

## Null checks

The builtin `is_null()` tests for `null`:

```novum
let value = null
print(is_null(value))
```
