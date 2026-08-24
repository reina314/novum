---
layout: page
title: Runtime Types
parent: Language Reference
---

# Runtime Types

The evaluator operates on a `Value` enum containing the language's runtime value categories. A parallel `Type` representation is used for expected-type helpers and diagnostics.

Important runtime distinctions include:

```text
Int
Float
Bool
Str
List
Dict
Tuple
Range
Vector
Matrix
Iterator
Object
Class
Module
Function
Builtin
EnumValue
EnumConstructor
Option / Result representations
Path
Unit
Null
```

Lists, vectors, and matrices use value-based equality. Objects are reference-backed runtime values and use their object semantics rather than collection pointer identity.

The exact `Type` enum should be kept synchronized with `Value`; helper APIs should prefer generic type conversion (`FromValue` / the generic expectation layer) over a separate hand-written helper for every type.

