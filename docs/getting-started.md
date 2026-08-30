---
layout: default
title: Getting Started
nav_order: 2
---

# Getting Started

This page covers the minimum workflow for running Novum programs.

## Build from source

The repository contains the Novum binary as a Cargo package. From the repository root, build the optimized executable with:

```bash
cargo build --release
```

The resulting binary is `target/release/novum`. During development, `cargo run` can be used in the same way:

```bash
cargo run -- program.nv
```

## Command-line usage

The executable accepts an optional source file and a small set of diagnostic flags.

```text
novum [OPTIONS] [FILE]
```

The currently implemented options are:

| Option | Meaning |
|---|---|
| `-h`, `--help` | Show command-line help |
| `-V`, `--version` | Show the Novum version |
| `-l`, `--lexer` | Show lexer output while running |
| `-p`, `--parser` | Show parser output while running |
| `-a`, `--all` | Show lexer and parser output |

Run a file with:

```bash
novum program.nv
```

Without a file, Novum starts its REPL:

```bash
novum
```

## The REPL

The VM REPL provides command history and normal line editing. The implemented editing keys include:

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate history |
| `←` / `→` | Move the cursor |
| `Home` / `End` | Move to the line boundary |
| `Shift+Enter` | Insert a new line |
| `Ctrl+Enter` | Insert a new line |
| `Ctrl-C` | Cancel the current input |
| `Ctrl-D` | Exit |

The REPL also recognizes `help`, `quit`, and `exit` as interactive commands.

## Your first program

Create `hello.nv`:

```novum
let name = "Novum"
print("Hello, " + name + "!")
```

Then run it:

```bash
novum hello.nv
```

## A small data example

```novum
let values = [1, 2, 3, 4, 5]

let doubled =
    values
        .map(|x| x * 2)
        .collect()

print(doubled)
```

This example already demonstrates the central Novum workflow: create a value, transform it with a lambda, and materialize a lazy pipeline only when a final list is needed.

## Importing a standard library module

Standard-library modules are loaded through `import`:

```novum
import math

print(math.sqrt(16))
```

Aliases are supported:

```novum
import math as m
print(m.pi())
```

See the [Standard Library](stdlib/index.md) reference for the complete module catalog.
