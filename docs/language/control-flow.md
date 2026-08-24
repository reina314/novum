---
layout: page
title: Control Flow
parent: Language Guide
---

# Control Flow

## `if`

```novum
if condition {
    value_a
} else {
    value_b
}
```

`if` is expression-oriented.

## `while`

```novum
while condition {
    work()
}
```

The condition must evaluate to `Bool`.

## `for`

`for` consumes any value that can be converted to an iterator:

```novum
for x in [1, 2, 3] {
    print(x)
}
```

Ranges and strings can also be used directly:

```novum
for x in (1..5) {
    print(x)
}

for ch in "hello" {
    print(ch)
}
```

## `break`

```novum
while true {
    if done {
        break
    }
}
```

## `continue`

```novum
for x in xs {
    if x < 0 {
        continue
    }
    print(x)
}
```

`continue` is loop-local control flow and must not escape a function/module boundary.

## `return`

```novum
f = |x| {
    if x < 0 {
        return 0
    }
    x
}
```

A `return` used where a value is required is rejected by the evaluator. Function calls preserve return flow correctly.

