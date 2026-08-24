---
layout: page
title: Data, JSON, and Numerical APIs
parent: Standard Library
---

# Data, JSON, and Numerical APIs

Novum's data-oriented runtime includes lists, dictionaries, vectors, matrices, and data-frame-oriented operations developed alongside the statistical layer.

## JSON

JSON support is intended to use the Rust ecosystem for parsing/serialization rather than reimplementing a JSON parser from scratch. The exact public module/function names should follow the JSON module implementation shipped in the repository.

A typical design is:

```novum
import json

let value = json.read(path("config.json"))?
```

When integrating JSON, preserve the distinction between JSON object/array/null values and Novum's native `Dict`, `List`, and `Null` runtime values.

## DataFrames

The data-oriented runtime includes DataFrame/Series functionality used for research and statistical workflows. Existing operations include column access and descriptive/statistical helpers.

Example style:

```novum
df.column("age")
df.column("score")
```

DataFrame and Series APIs remain under active expansion and should be documented from their public module implementation once their final method catalog stabilizes.

## Linear algebra

Matrices and vectors are part of the numerical runtime. Matrix convenience operations are increasingly integrated into the matrix API, while more specialized algorithms remain available through the linear-algebra module.

