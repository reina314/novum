---
layout: page
title: Module Loading Internals
parent: Design and Internals
---

# Module Loading Internals

Novum keeps file execution state and module semantic state separate.

## File stack

The file stack records the physical source files currently being evaluated. Its primary purpose is relative import resolution.

```text
main.nv
  ↓
modules/a.nv
  ↓
modules/sub/b.nv
```

## Module stack

The module stack records module contexts used for:

- cyclic-import detection;
- public export tracking;
- module identity and diagnostics.

## Module loader

`ModuleLoader` is responsible for resolving and parsing physical source files and caching successfully evaluated modules. It should not own interpreter execution state.

## Resolution order

For a user module import:

1. Search relative to the importing file.
2. Search relative to the configured project/root directory.
3. If the import has one path component and no physical module was found, try a standard-library module.

A canonical physical path is used as the cache/cycle identity.

## Aliases

Aliases change only the environment binding:

```text
import tests.modules.math as m
```

The requested module identity remains `tests.modules.math`.

