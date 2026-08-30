---
layout: default
title: Operators
parent: Reference
nav_order: 2
---

# Operators

## Arithmetic

| Operator | Meaning |
|---|---|
| `+` | Addition |
| `-` | Subtraction / unary negation |
| `*` | Multiplication |
| `/` | Division |
| `%` | Remainder |
| `**` | Exponentiation |
| `@` | Matrix multiplication |

Compound assignment forms are also available:

```text
+=  -=  *=  /=  %=
```

## Comparison

```text
==  !=  <  <=  >  >=
```

`is` is accepted as an alias for equality (`==`).

## Logical operators

```text
and
or
not
```

## Range operators

```text
..     exclusive end
..=    inclusive end
```

Examples:

```novum
1..5
1..=5
```
