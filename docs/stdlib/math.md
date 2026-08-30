---
layout: default
title: Math
parent: Standard Library
nav_order: 2
---

# `math`

Import with:

```novum
import math
```

All mathematical arguments may be `Int` or `Float`; the returned numeric result is a `Float` unless stated otherwise.

## Basic numeric functions

| Function | Description |
|---|---|
| `math.abs(x)` | Absolute value |
| `math.sign(x)` | Sign of `x` (`-1`, `0`, or `1` for ordinary finite values) |
| `math.floor(x)` | Largest integer-valued `Float` not greater than `x` |
| `math.ceil(x)` | Smallest integer-valued `Float` not less than `x` |
| `math.round(x)` | Round to the nearest integer-valued `Float` using Rust's floating-point rounding semantics |
| `math.trunc(x)` | Remove the fractional part toward zero |
| `math.fract(x)` | Fractional part |

Examples:

```novum
math.abs(-3.5)
math.floor(3.9)
math.ceil(3.1)
math.trunc(-3.9)
math.fract(3.25)
```

## Powers, roots, and logarithms

| Function | Description |
|---|---|
| `math.sqrt(x)` | Square root |
| `math.cbrt(x)` | Cube root |
| `math.pow(x, y)` | `x` raised to the power `y` |
| `math.exp(x)` | `e^x` |
| `math.exp2(x)` | `2^x` |
| `math.ln(x)` | Natural logarithm |
| `math.log(x)` | Natural logarithm (same implementation as `ln`) |
| `math.log2(x)` | Base-2 logarithm |
| `math.log10(x)` | Base-10 logarithm |

```novum
math.sqrt(16)
math.pow(2, 8)
math.log10(1000)
```

## Trigonometric functions

| Function | Description |
|---|---|
| `math.sin(x)` | Sine, radians |
| `math.cos(x)` | Cosine, radians |
| `math.tan(x)` | Tangent, radians |
| `math.asin(x)` | Inverse sine |
| `math.acos(x)` | Inverse cosine |
| `math.atan(x)` | Inverse tangent |
| `math.atan2(y, x)` | Four-quadrant inverse tangent |

```novum
let angle = math.pi() / 4
print(math.sin(angle))
```

## Hyperbolic functions

| Function | Description |
|---|---|
| `math.sinh(x)` | Hyperbolic sine |
| `math.cosh(x)` | Hyperbolic cosine |
| `math.tanh(x)` | Hyperbolic tangent |
| `math.asinh(x)` | Inverse hyperbolic sine |
| `math.acosh(x)` | Inverse hyperbolic cosine |
| `math.atanh(x)` | Inverse hyperbolic tangent |

## Numeric utilities

| Function | Description |
|---|---|
| `math.hypot(x, y)` | Euclidean norm `sqrt(x² + y²)` |
| `math.min(x, y)` | Minimum of two numeric values |
| `math.max(x, y)` | Maximum of two numeric values |
| `math.clamp(x, min, max)` | Clamp `x` into `[min, max]` |

Example:

```novum
math.clamp(12, 0, 10)    // 10.0
math.hypot(3, 4)          // 5.0
```

## Mathematical constants

The constants are exposed as zero-argument functions:

```novum
math.pi()
math.e()
math.tau()
```

They return the corresponding floating-point constants.
