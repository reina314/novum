---
layout: page
title: Iterator Architecture
parent: Design and Internals
---

# Iterator Architecture

The iterator system is designed around a conversion boundary:

```text
Value
  ↓
IteratorObj::from_value
  ↓
IteratorRef
```

Iterable values can therefore participate in iterator methods without exposing `.iter()` as mandatory user syntax.

## Method dispatch

When field access requests a method:

```text
receiver supports native method?
    yes → native bound method
    no  → is this an iterator method?
              yes → implicit iterable-to-iterator conversion
              no  → method-not-found error
```

## Lazy adaptors

`map`, `filter`, `enumerate`, `zip`, `take`, and `skip` are represented as iterator objects. They should not eagerly materialize the underlying sequence.

`collect()` is the explicit materialization boundary.

## `zip`

`zip` accepts ordinary iterable values because its arguments are normalized through the same `Value → Iterator` conversion used elsewhere.

## Extension point

Adding a new iterable runtime type should normally require one new conversion arm in `IteratorObj::from_value()` rather than a collection of type-specific evaluator branches.

