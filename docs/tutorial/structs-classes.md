---
layout: default
title: Structs and Classes
parent: Tutorial
nav_order: 5
---

# Structs and Classes

Novum supports both `struct` and `class`. For everyday programming, think of them as named object types with fields, defaults, methods, and optional constructors.

## A simple struct

```novum
struct Point {
    x
    y
}

let p = Point(2, 3)
print(p.x)
```

## Default field values

```novum
struct Point {
    x = 0
    y = 0
}

let a = Point()
let b = Point(5)
let c = Point(5, 8)
```

Fields without supplied values use their defaults. A field with neither a value nor a default must be supplied during construction.

## Methods

Methods are lambdas declared inside the type. The first parameter is `self`:

```novum
struct Point {
    x = 0
    y = 0

    move = |self, dx, dy| {
        self.x += dx
        self.y += dy
    }
}

let p = Point(1, 2)
p.move(3, 4)
```

When you call `p.move(...)`, Novum supplies `self` automatically.

## Classes

`class` uses the same member syntax:

```novum
class Counter {
    value = 0

    increment = |self| {
        self.value += 1
    }
}

let c = Counter()
c.increment()
```

## Constructors with `init`

Define an explicit constructor with the `init` member:

```novum
class Point {
    x = 0
    y = 0

    init = |self, x, y| {
        self.x = x
        self.y = y
    }
}

let p = Point(10, 20)
```

Defaults are established before `init` executes, so the constructor can safely modify already-initialized fields.

## Named constructor arguments

When `init` exists, its parameter names are also accepted as named constructor arguments:

```novum
let p = Point(y=20, x=10)
```

Without `init`, named arguments correspond to field names.

## Public types

Top-level structs and classes can be exported:

```novum
pub struct Point {
    x
    y
}

pub class Counter {
    value = 0
}
```

Use `pub` when another module should be able to import the declaration.
