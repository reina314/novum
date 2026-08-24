---
layout: page
title: Modules and Visibility
parent: Language Guide
---

# Modules and Visibility

## Importing modules

```novum
import math
```

Nested module paths are supported:

```novum
import tests.modules.statistics
```

Without an alias, the requested module participates in the existing nested module namespace:

```novum
tests.modules.statistics.mean(data)
```

## Import aliases

```novum
import tests.modules.statistics as stats

stats.mean(data)
```

The alias binds the requested module directly under the alias name rather than creating a nested namespace for that import.

Aliases are also supported for standard-library modules:

```novum
import math as m
m.sqrt(16)
```

## Relative module resolution

When a source file imports another module, Novum first searches relative to the directory of the importing file. Project-root resolution is used as a fallback. A single-component import can then resolve to a standard-library module when no physical user module is found.

For example:

```text
samples/
├── test1.nv
└── test2.nv
```

`test1.nv` can use:

```novum
import test2
```

File execution tracks the source-file stack separately from the module stack so nested relative imports continue to resolve correctly.

## Visibility

`pub` is meaningful for top-level declarations:

```novum
pub let value = 42
pub struct Point { x, y }
pub class Counter { value = 0 }
```

Non-public top-level declarations remain private to the module.

## Cyclic imports

Novum detects cyclic module imports using canonical physical file paths and reports the import chain.

## Standard library loading

The standard library's builtins can be installed eagerly at interpreter startup, while standard-library modules are lazy-loaded when imported. User modules are loaded lazily as well.

