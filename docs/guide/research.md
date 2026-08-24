---
layout: page
title: Research-Oriented Programming
parent: Practical Guides
---

# Research-Oriented Programming

Novum's data and numerical facilities are designed to support compact analysis scripts.

## Data transformation

Use list/string/iterator pipelines for preprocessing:

```novum
let cleaned =
    values
        .filter(|x| x != Null)
        .map(|x| float(str(x)))
        .collect()
```

Use vector and matrix values when numerical operations naturally map to linear algebra.

## Statistics

The project contains statistical functionality developed as part of its research-oriented feature set, including descriptive operations and hypothesis-testing utilities. The exact catalog is implementation-driven and should be kept synchronized with the statistics module.

## Reproducible scripts

Prefer explicit input paths and configuration rather than depending on the REPL environment:

```novum
import process
import fs

let root = process.cwd()?
let input = root.join("data/input.csv")
let text = fs.read(input)?
```

## Why functions are lambdas

Research workflows benefit from passing transformations directly:

```novum
column
    .map(|x| transform(x))
    .filter(|x| predicate(x))
```

This keeps small analytical transformations close to the operation they parameterize.

