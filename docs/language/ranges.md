---
layout: page
title: Ranges
parent: Language Guide
---

# Ranges

## Exclusive ranges

```novum
1..5
```

produces the values `1, 2, 3, 4` when iterated.

## Inclusive ranges

```novum
1..=5
```

produces `1, 2, 3, 4, 5`.

## Ranges in loops

```novum
for x in (1..=5) {
    print(x)
}
```

## Ranges in list construction

The list evaluator can consume range items when the range syntax is used in a list context supported by the parser:

```text
[1, 2, 3, ...range syntax supported by the current grammar]
```

The exact surface form should follow the parser version used by the repository.

## Implementation note

The iterator runtime represents a range with an exclusive endpoint. Inclusive syntax is normalized before constructing the iterator. This keeps iterator advancement simple and consistent.

