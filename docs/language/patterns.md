---
layout: page
title: Pattern Matching
parent: Language Guide
---

# Pattern Matching

Novum supports pattern-based binding and `match` expressions.

## Wildcard

```novum
match value {
    _ => 0
}
```

## Identifier pattern

```novum
match value {
    x => x
}
```

## Literal patterns

```novum
match value {
    0 => "zero"
    1 => "one"
    _ => "other"
}
```

## Tuple patterns

```novum
match pair {
    (x, y) => x + y
    _ => 0
}
```

## Enum variant patterns

Unit variant:

```novum
match color {
    Color.Green => 1
    _ => 0
}
```

Payload variant:

```novum
match result {
    Result.Ok(value) => value
    Result.Err(message) => print(message)
}
```

Pattern parser support includes qualified enum paths such as `Color.Green` and payload patterns such as `Result.Ok(value)`.

## List patterns

List-pattern support is part of the current language implementation and is intended for destructuring list values. The exact accepted syntax should follow the parser/tests shipped with the current revision.

