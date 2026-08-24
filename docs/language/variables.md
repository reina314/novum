---
layout: page
title: Variables and Assignment
parent: Language Guide
---

# Variables and Assignment

Novum intentionally keeps declaration and assignment syntax compact.

## Ordinary assignment-style binding

```novum
x = 10
x = 20
```

A name can be introduced by assignment when appropriate. The evaluator distinguishes declaration semantics when an explicit declaration is requested.

## `let`

```novum
let x = 10
```

`let` is useful for explicit declarations, pattern binding, and visibility-aware declarations:

```novum
pub let answer = 42
```

## Patterns in `let`

Pattern binding can destructure values:

```novum
let (x, y) = pair
```

Patterns can include wildcards, identifiers, literals, tuples, enums, and lists where supported by the current parser/evaluator.

## Assignment targets

The following are assignment targets:

```novum
x = 10
xs[0] = 10
d["value"] = 10
object.value = 10
```

Compound assignment uses the same targets:

```novum
x += 1
xs[0] += 1
object.value += 1
```

Compound assignment evaluates the target once conceptually: read current value, apply the binary operator, then write the result.

