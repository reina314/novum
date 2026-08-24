---
layout: page
title: Structs and Classes
parent: Language Guide
---

# Structs and Classes

Novum exposes both `struct` and `class` syntax. Internally, both are represented by a runtime `Class` definition and instantiated as runtime `Object` values.

## Struct syntax

```novum
struct Point {
    x
    y
}
```

A struct without an explicit `init` can be instantiated positionally:

```novum
let p = Point(2, 3)
```

## Default field values

```novum
struct Point {
    x = 0
    y = 0
}
```

Then:

```novum
Point()
Point(5)
Point(5, 3)
```

use defaults for fields that were not initialized positionally. A field with neither a supplied value nor a default is an error.

## Methods

Methods are lambdas assigned inside the type declaration. The first parameter must be `self`:

```novum
struct Point {
    x = 0
    y = 0

    move = |self, dx, dy| {
        self.x += dx
        self.y += dy
    }
}
```

Then:

```novum
let p = Point(1, 2)
p.move(5, 3)
```

## Classes

Classes use the same member syntax:

```novum
class Counter {
    value = 0

    increment = |self| {
        self.value += 1
    }
}
```

## Constructors (`init`)

An `init` member is treated as the constructor:

```novum
class Point {
    x = 0
    y = 0

    init = |self, x, y| {
        self.x = x
        self.y = y
    }
}
```

Construction:

```novum
let p = Point(5, 3)
```

Field defaults are established before `init` runs, so constructor code can safely mutate defaulted fields:

```novum
class Perceptron {
    weights = [0.0]
    bias = 0.0

    init = |self, input_dim| {
        self.weights *= input_dim
    }
}
```

## Named constructor arguments

When `init` exists, named arguments correspond to the `init` parameter names:

```novum
Perceptron(
    input_dim = 2,
    learning_rate = 0.5,
    epochs = 20
)
```

When no `init` exists, named arguments correspond to field names:

```novum
struct Point {
    x
    y
}

Point(
    y = 3,
    x = 2
)
```

## `self`

`self` is supplied by the bound method/constructor call and must be the first lambda parameter. It cannot be supplied explicitly as a named argument.

## Visibility

Top-level declarations can be public:

```novum
pub struct Point {
    x
    y
}

pub class Counter {
    value = 0
}
```

`pub struct` / `pub class` are valid only at top-level. Imported modules use this information to construct their exported interface.

