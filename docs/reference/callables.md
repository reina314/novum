---
layout: page
title: Callable Semantics
parent: Language Reference
---

# Callable Semantics

Novum currently has several callable categories.

## User-defined functions

Functions are lambda values. They support positional and named arguments.

## Bound methods

Object methods are bound to a receiver. The receiver is injected as `self` and therefore is not supplied by the caller.

## Classes

A runtime `Class` value is callable. Calling a class creates an `Object` instance and then either:

1. invokes `init(self, ...)` when an explicit constructor exists, or
2. maps positional/named arguments to fields when no constructor exists.

Field defaults are available before an explicit constructor runs.

## Builtins

Builtins currently expose a `Vec<Value>`-style host function interface. Named arguments are deliberately rejected until builtin parameter metadata is introduced.

## Native methods

Native methods on types such as `Str`, `List`, `Dict`, `Iterator`, and `Path` currently accept positional arguments. User-defined object methods support named arguments.

## Enum constructors

Enum constructors currently use positional arguments and validate the number of payload values against the variant arity.

