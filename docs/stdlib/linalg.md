---
layout: default
title: Linear Algebra
parent: Standard Library
nav_order: 7
---

# `linalg`

Import with:

```novum
import linalg
```

The module works with `Vector` and `Matrix` values and provides core numerical linear-algebra operations.

## Constructors

### `linalg.vector(values)`

Creates a numeric `Vector` from a `List` of `Int` or `Float` values.

```novum
let v = linalg.vector([3, 4])
```

### `linalg.matrix(rows)`

Creates a numeric `Matrix` from a nested `List`:

```novum
let A = linalg.matrix([
    [1, 2],
    [3, 4]
])
```

Every row must be a list of numeric values and all rows must form a rectangular matrix.

## Basic matrix functions

### `linalg.transpose(A)`

Returns the transpose of a matrix.

```novum
linalg.transpose(A)
```

Matrices also expose a `transpose()` method:

```novum
A.transpose()
```

### `linalg.shape(A)`

Returns a two-element `List` containing `[rows, cols]`.

### `linalg.rows(A)` / `linalg.cols(A)`

Return the matrix dimensions separately as `Int` values.

```novum
print(A.shape())
print(linalg.rows(A))
print(linalg.cols(A))
```

## Matrix algebra

The matrix runtime supports the `@` operator for matrix multiplication:

```novum
let C = A @ B
```

### `linalg.det(A)`

Returns the determinant as a `Float`.

```novum
let d = linalg.det(A)
```

### `linalg.inverse(A)`

Returns the inverse matrix. The operation fails when the matrix is not invertible or does not satisfy the requirements of the underlying algorithm.

```novum
let A_inv = linalg.inverse(A)
```

## Solving systems

### `linalg.solve(A, b)`

Solves the linear system `A x = b`.

The right-hand side `b` may be a `Vector` or a `Matrix`:

```novum
let A = linalg.matrix([
    [2, 1],
    [1, 3]
])

let b = linalg.vector([4, 5])
let x = linalg.solve(A, b)
```

The result has the same general container category as `b`: a `Vector` for a vector right-hand side or a `Matrix` for a matrix right-hand side.

### `linalg.solve_lstsq(A, b)`

Computes a least-squares solution using the same `Vector` / `Matrix` right-hand-side conventions as `solve()`.

```novum
let coefficients = linalg.solve_lstsq(X, y)
```

## Linear regression

### `linalg.linear_regression(X, y)`

Fits a least-squares linear model.

`X` must be a `Matrix`. `y` can be either a `Vector` or a one-column `Matrix`.

```novum
let X = linalg.matrix([
    [1, 10],
    [1, 12],
    [1, 15],
    [1, 20]
])

let y = linalg.vector([12, 14, 17, 22])

let fit = linalg.linear_regression(X, y)
```

The returned value is a `Dict` with these properties:

| Property | Type | Meaning |
|---|---|---|
| `coefficients` | `Matrix` | Least-squares coefficient matrix |
| `fitted` | `Matrix` | Predicted values `X @ coefficients` |
| `r_squared` | `Float` | Coefficient of determination |
| `residual_sum_of_squares` | `Float` | Sum of squared residuals |

Access a property with normal dictionary indexing:

```novum
print(fit["coefficients"])
print(fit["r_squared"])
```

## Vector convenience methods

Vectors support numerical operations through the runtime and expose:

### `v.norm()`

Returns the Euclidean norm of the vector.

```novum
let v = linalg.vector([3, 4])
print(v.norm())   // 5.0
```

Matrices similarly support common object-level helpers such as `transpose()` and `shape()`, while more specialized algorithms are provided by the `linalg` module.
