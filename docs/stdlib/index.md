---
layout: default
title: Standard Library
nav_order: 4
has_children: true
permalink: /stdlib/
---

# Standard Library

Novum's standard library is split between eager builtins and lazily loaded modules.

## Builtins

Builtins are available without `import`:

```novum
print(typeof(42))
let values = range(10)
```

See [Builtins](builtin.md).

## Standard modules

The `vm` branch currently exposes these modules:

| Module | Purpose |
|---|---|
| [`math`](math.md) | Numeric, transcendental, and mathematical utility functions |
| [`csv`](csv.md) | Read CSV files into DataFrames |
| [`json`](json.md) | Parse and serialize JSON |
| [`fs`](fs.md) | File and directory operations |
| [`process`](process.md) | Environment variables, command-line arguments, and child processes |
| [`linalg`](linalg.md) | Vectors, matrices, linear systems, and linear regression |

Import a module with:

```novum
import math
import csv as c
```

Each module page documents the public functions exposed by the current implementation, including argument types and returned values.
