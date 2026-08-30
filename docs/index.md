---
layout: home
title: Novum
nav_order: 1
permalink: /
---

# Novum

> **Quick ideas. Quick experiments. Quick results.**

Novum is a small, practical programming language that combines compact syntax, expression-oriented programming, lazy iterators, data-oriented values, and a small standard library for research and scripting tasks.

## What is Novum?

Novum is designed for programs where the language should stay out of the way. Its syntax is intentionally compact, while the runtime provides useful building blocks for manipulating collections, working with files, invoking external processes, and performing numerical computation.

The current `vm` development line is the implementation described by these pages. The language is still evolving, so this documentation follows the behavior that exists in the repository rather than describing a separate or idealized specification.

## Start here

| Section | What you will find |
|---|---|
| [Getting Started](getting-started.md) | Installation, command-line usage, and the REPL |
| [Tutorial](tutorial/index.md) | Language basics through classes and iterator pipelines |
| [Standard Library](stdlib/index.md) | Builtins and the seven current standard modules |
| [Reference](reference/index.md) | Types, operators, calls, and iteration at a glance |

## Why Novum?

### Compact by default

A function is simply a lambda expression bound to a name:

```novum
square = |x| x * x
```

Blocks are expressions, so small transformations can remain visually close to the code that uses them.

### Iteration as a first-class workflow

Sequences can be transformed through lazy iterator pipelines:

```novum
let result =
    (1..100)
        .filter(|x| x % 2 == 0)
        .map(|x| x * x)
        .take(10)
        .collect()
```

The pipeline stays lazy until a terminal operation such as `collect()` is reached.

### Useful for data work

Novum has runtime values for `Series`, `DataFrame`, `Vector`, and `Matrix`, alongside builtins for constructing and inspecting data-oriented values.

### Small scripts can still touch the system

The standard library includes `fs` for files and directories, `process` for environment variables and external commands, and `json` / `csv` for common data exchange formats.

## Documentation map

| Section | Purpose |
|---|---|
| [Getting Started](getting-started.md) | Installation, command-line usage, and the REPL |
| [Tutorial](tutorial/index.md) | Learn Novum progressively with runnable examples |
| [Standard Library](stdlib/index.md) | User-facing API reference for the standard modules |
| [Reference](reference/index.md) | Operators, types, and compact language reference |

> **Note**
>
> The documentation intentionally does **not** describe bytecode, compiler internals, VM implementation details, or Rust data structures. It is written from a user perspective.