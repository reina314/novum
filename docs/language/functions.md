---
layout: page
title: Functions and Lambdas
parent: Language Guide
---

# Functions and Lambdas

Novum represents user-defined functions as lambda expressions. There is no separate required `fn` declaration syntax for ordinary functions.

## Lambda syntax

Expression body:

```novum
square = |x| x * x
```

Block body:

```novum
compute = |x| {
    let y = x * 2
    y + 1
}
```

Multiple parameters:

```novum
add = |x, y| x + y
```

## Calling lambdas

```novum
add(2, 3)
```

## Named arguments

Named arguments work with user-defined lambdas/functions:

```novum
add(y=3, x=2)
```

Mixed calls are also supported as long as positional arguments come first:

```novum
add(2, y=3)
```

## Argument rules

- Unknown parameter names are errors.
- Supplying one parameter more than once is an error.
- Positional arguments after a named argument are errors.
- Missing required parameters are errors.
- `self` is injected automatically for bound methods and cannot be supplied explicitly as a named argument.

## Higher-order functions

Because functions are values, they can be passed to iterator methods:

```novum
[1, 2, 3]
    .map(|x| x * 2)
    .collect()
```

## Recursion

A lambda bound to a name can refer to that name recursively when the environment semantics permit it:

```novum
factorial = |n| {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
```

