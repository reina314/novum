---
layout: default
title: Modules, Enums, and Pattern Matching
parent: Tutorial
nav_order: 6
---

# Modules, Enums, and Pattern Matching

## Import a module

```novum
import math

print(math.sqrt(25))
```

Use an alias when you want a shorter name:

```novum
import math as m
print(m.sqrt(25))
```

User modules can also be imported with nested paths.

## Module visibility

Top-level declarations are private by default. `pub` makes them visible to importers:

```novum
pub let answer = 42

pub square = |x| x * x
```

A module therefore has a clear public surface instead of exposing every internal binding.

## Enums

Novum supports enum declarations:

```novum
enum Color {
    Red
    Green
    Blue
}
```

Variants can also carry fields:

```novum
enum Result {
    Ok
    Err
}
```

The language runtime also provides the standard `Option` and `Result` enum families used by the standard library.

## `match`

Pattern matching is expression-oriented:

```novum
let label = match value {
    0 => "zero"
    1 => "one"
    _ => "other"
}
```

Qualified enum patterns are supported:

```novum
match color {
    Color.Green => "go"
    Color.Red => "stop"
    _ => "unknown"
}
```

Payload patterns can bind the contained value:

```novum
match result {
    Result.Ok(value) => value
    Result.Err(message) => print(message)
}
```

## Destructuring with patterns

Patterns can also be used by `let`:

```novum
let (x, y) = (10, 20)
```

The same pattern family includes wildcard, identifier, literal, tuple, list, enum, and struct forms.
