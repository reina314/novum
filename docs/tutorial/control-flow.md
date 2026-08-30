---
layout: default
title: Control Flow
parent: Tutorial
nav_order: 3
---

# Control Flow

## `if` / `else`

`if` is an expression:

```novum
let label = if score >= 60 {
    "pass"
} else {
    "fail"
}
```

The condition must evaluate to `Bool`.

## `while`

```novum
let n = 0

while n < 5 {
    print(n)
    n += 1
}
```

## `for`

`for` consumes an iterable directly:

```novum
for x in [1, 2, 3] {
    print(x)
}
```

Ranges and strings work too:

```novum
for x in (1..5) {
    print(x)
}

for ch in "Novum" {
    print(ch)
}
```

## `break` and `continue`

```novum
for x in (0..10) {
    if x == 5 {
        break
    }
    print(x)
}
```

```novum
for x in (0..10) {
    if x % 2 != 0 {
        continue
    }
    print(x)
}
```

## `return`

Use `return` for an early function result:

```novum
absolute = |x| {
    if x < 0 {
        return -x
    }
    x
}
```

## Ranges

Exclusive range:

```novum
1..5
```

Iterates as `1, 2, 3, 4`.

Inclusive range:

```novum
1..=5
```

Iterates as `1, 2, 3, 4, 5`.

Ranges are particularly useful with `for`, `iter`, and iterator pipelines.
