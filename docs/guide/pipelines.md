---
layout: page
title: Functional Data Pipelines
parent: Practical Guides
---

# Functional Data Pipelines

Iterator auto-conversion makes list, string, and range pipelines concise.

```novum
let result =
    [1, 2, 3, 4, 5]
        .filter(|x| x % 2 == 1)
        .map(|x| x * x)
        .collect()
```

## Zip multiple datasets

```novum
let pairs =
    zip(
        ["Alice", "Bob"],
        [10, 20]
    )
    .collect()
```

## Enumerate text

```novum
let chars =
    "Novum"
        .enumerate()
        .collect()
```

## Bounded range processing

```novum
let first_ten =
    (1..=100)
        .filter(|x| x % 2 == 0)
        .take(10)
        .collect()
```

## Materialize only at the boundary

Iterator adaptors are lazy, so a good pipeline keeps the value as an iterator until a concrete list is required:

```novum
let pipeline =
    input
        .filter(predicate)
        .map(transform)
        .take(100)

let values = pipeline.collect()
```

