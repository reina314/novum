---
layout: default
title: Calls and Named Arguments
parent: Reference
nav_order: 3
---

# Calls and Named Arguments

Novum function calls accept positional and named arguments.

## Positional calls

```novum
add(2, 3)
Point(10, 20)
```

## Named calls

```novum
Point(y=20, x=10)
```

## Mixed calls

Positional arguments must come before named arguments:

```novum
f(1, y=2)     // valid
f(x=1, 2)     // invalid
```

Unknown parameter names, duplicate arguments, and missing required parameters are errors.

For bound methods, `self` is injected automatically and must not be passed explicitly.
