---
layout: page
title: Operators
parent: Language Guide
---

# Operators

Novum provides ordinary arithmetic, comparison, logical, indexing, member-access, range, assignment, and compound-assignment operators.

## Arithmetic

Typical arithmetic forms include:

```novum
x + y
x - y
x * y
x / y
x % y
```

## Repetition

Lists and strings support multiplication by an integer count:

```novum
[0] * 5
```

produces a list equivalent to:

```text
[0, 0, 0, 0, 0]
```

String repetition is useful for formatting:

```novum
print("=" * 20)
```

Sequence repetition is distinct from numeric vector/matrix scalar multiplication.

## Comparisons

```novum
x == y
x != y
x < y
x <= y
x > y
x >= y
```

Lists use element-wise equality rather than pointer identity. Vectors and matrices likewise compare their values element-wise.

## Logical operators

```novum
a and b
a or b
```

Logical evaluation is short-circuiting and expects boolean operands for the scalar case.

## Assignment

```novum
x = 10
x += 1
x -= 1
x *= 2
x /= 2
x %= 2
```

Compound assignment also works with index and field targets:

```novum
xs[0] += 1
obj.value += 1
```

## Indexing

```novum
xs[0]
d["key"]
```

## Field access

```novum
obj.field
```

## Range operators

Exclusive range:

```novum
1..5
```

Inclusive range:

```novum
1..=5
```

The iterator representation normalizes inclusive ranges to an exclusive endpoint internally.

## Postfix `?`

`?` propagates supported `Option`/`Result` failure values out of an expression.

```novum
let text = fs.read(path("data.txt"))?
```

