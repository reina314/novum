---
layout: default
title: Functions and Lambdas
parent: Tutorial
nav_order: 2
---

# Functions and Lambdas

Functions in Novum use the same compact lambda syntax everywhere. There is no separate required `fn` declaration for ordinary functions.

## One parameter

```novum
square = |x| x * x

print(square(5))
```

## Several parameters

```novum
add = |x, y| x + y

print(add(2, 3))
```

## Block-bodied lambdas

```novum
describe = |x| {
    let doubled = x * 2
    print("value = " + str(x))
    doubled
}
```

The last expression becomes the result of the function.

## Named arguments

Calls can mix positional and named arguments, but positional arguments must come first:

```novum
combine = |x, y| x * 10 + y

combine(2, y=3)
combine(y=3, x=2)
```

This is especially useful for constructors and functions with several configuration parameters.

## Functions are values

A function can be stored in a variable or passed to another function:

```novum
double = |x| x * 2
apply = |f, x| f(x)

print(apply(double, 10))
```

## Recursion

A named lambda can call itself:

```novum
factorial = |n| {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

print(factorial(5))
```

## Functions and iterators

This is where Novum's lambda syntax becomes particularly useful:

```novum
let result =
    [1, 2, 3, 4]
        .map(|x| x * x)
        .filter(|x| x > 5)
        .collect()
```

The callback syntax remains small enough that the transformation itself stays readable.
