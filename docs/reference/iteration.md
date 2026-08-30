---
layout: default
title: Iteration at a Glance
parent: Reference
nav_order: 4
---

# Iteration at a Glance

The following values can currently be converted into iterators:

```text
Iterator
List
Str
Range
Dict
Vector
Series
DataFrame
```

Core lazy transformations:

```text
map(callback)
filter(callback)
take(n)
skip(n)
enumerate()
zip(other)
```

Terminal / reducing operations:

```text
collect()
reduce(callback)
fold(initial, callback)
any(callback)
all(callback)
```

The usual pattern is:

```novum
source
    .filter(...)
    .map(...)
    .take(...)
    .collect()
```

Use the [Tutorial's iterator chapter](../tutorial/iterators.md) for detailed examples.
