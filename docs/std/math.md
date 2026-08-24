---
layout: page
title: Math
parent: Standard Library
---

# Math

The math module provides numerical functions and is imported lazily:

```novum
import math
```

The runtime has included fundamental operations such as `sqrt` and `abs`, together with an expanding numerical function set.

Examples:

```novum
import math

math.sqrt(9)
math.abs(-3)
```

The broader numerical/linear-algebra API includes statistical and matrix functionality. Because the module surface is actively evolving, this page intentionally distinguishes stable language-level semantics from implementation-specific function catalogs.

For matrix functionality also see the integrated methods described in the language guide, such as:

```novum
matrix.transpose()
matrix.shape()
```

