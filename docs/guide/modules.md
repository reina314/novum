---
layout: page
title: Modules and Project Layout
parent: Practical Guides
---

# Modules and Project Layout

A practical project can use source-relative modules:

```text
project/
├── main.nv
├── config.nv
├── data/
│   └── load.nv
└── models/
    └── user.nv
```

From `main.nv`:

```novum
import config
import data.load as loader
import models.user as user
```

This style keeps deeply nested project namespaces readable without sacrificing module identity.

## Public interfaces

Export only the names intended for consumers:

```novum
pub let version = "0.1"

pub class Model {
    value = 0
}

let internal_helper = |x| x + 1
```

Consumers can use the public names while private implementation details remain inaccessible through the module interface.

