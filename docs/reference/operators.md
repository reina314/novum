---
layout: page
title: Operator Reference
parent: Language Reference
---

# Operator Reference

The exact precedence table follows the recursive-descent/Pratt-style expression parser shipped with the implementation. The following categories are currently supported:

| Category | Examples |
|---|---|
| Assignment | `=` |
| Compound assignment | `+=`, `-=`, `*=`, `/=`, `%=` and other implemented compound forms |
| Logical | `and`, `or` |
| Comparison | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| Arithmetic | `+`, `-`, `*`, `/`, `%` |
| Member access | `.` |
| Indexing | `[...]` |
| Call | `(...)` |
| Range | `..`, `..=` |
| Try propagation | postfix `?` |

Assignment has lower precedence than ordinary arithmetic/logical expressions, allowing expressions such as:

```novum
x = 1 + 2 * 3
```

to parse as the assignment of `7`.

