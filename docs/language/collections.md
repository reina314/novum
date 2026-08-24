---
layout: page
title: Collections
parent: Language Guide
---

# Collections

## Lists

List literal:

```novum
xs = [1, 2, 3]
```

Core methods currently include:

```text
push(value)
pop()
remove(index)
len()
iter()
```

Example:

```novum
xs.push(4)
let x = xs.pop()
xs.remove(0)
xs.len()
```

Lists can be repeated:

```novum
[0] * 5
```

## Dictionaries

Dictionaries are mutable key/value collections. Dictionary indexing and field/method APIs are available through the runtime `Dict` receiver.

```novum
d = {
    "name": "Novum",
    "version": "dev"
}

print(d["name"])
```

Dictionary iteration exposes key/value pairs.

## Strings

Strings are UTF-8 text values. Current string methods include:

```text
chars()
len()
trim()
to_upper()
to_lower()
contains(value)
```

Strings are also directly iterable by the iterator protocol.

## Vectors

A list can be converted to a vector:

```novum
v = [1, 2, 3].vector()
```

Vectors support numeric operations such as vector/scalar and vector/matrix operations where defined by the runtime, as well as:

```text
norm()
```

## Matrices

Matrices support numerical operations and integrated convenience methods such as:

```text
transpose()
shape()
```

Additional linear-algebra operations remain available through the linear algebra library/API.

## Equality

Collection equality is value-based. List, vector, and matrix values are compared element-wise.

