---
layout: page
title: Module Semantics
parent: Language Reference
---

# Module Semantics

Module identity is based on the requested module path and canonical physical file path. Aliases affect environment bindings, not module identity, cache keys, or cyclic-import detection.

```text
import a.b.c
```

and

```text
import a.b.c as mod
```

refer to the same physical module when resolved to the same canonical file.

A successful module is cached so repeated imports do not re-evaluate its source.

Module execution occurs in a child environment. Exported names are recorded while evaluating top-level public declarations and are copied into the runtime `Module` interface after execution.

