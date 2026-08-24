---
layout: page
title: Getting Started
---

# Getting Started

## Running Novum from the repository

The normal development workflow is based on Cargo:

```text
cargo run
cargo test
```

To execute a source file, pass it to the Novum executable. A typical project layout is:

```text
project/
├── Cargo.toml
└── samples/
    ├── test1.nv
    └── test2.nv
```

```text
cargo run samples/test1.nv
```

The entry file participates in file-relative module resolution, so a file can import another `.nv` file located beside it:

```novum
import test2
```

## REPL

The REPL evaluates source strings directly and supports interactive expression evaluation. The repository implementation uses the same lexer, parser, and evaluator pipeline as file execution, while file execution additionally tracks the current source path for relative imports.

## A first program

```novum
x = 10
y = 20

print(x + y)
```

Variable declaration and assignment use the same basic syntax. `let` is available when explicit declaration syntax or public declaration semantics are desired, but ordinary assignment-style binding is intentionally concise.

## Functions are lambdas

Novum represents user-defined functions with lambda expressions:

```novum
add = |x, y| x + y

add(2, 3)
```

A lambda can have a block body:

```novum
factorial = |n| {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
```

## Errors and `?`

Novum has `Option` and `Result`-style values and supports postfix `?` propagation. For example:

```novum
let text = fs.read(path("data.txt"))?
```

A `Result.Err(...)` or `Option.None` can propagate out of an expression that permits the corresponding control flow.

