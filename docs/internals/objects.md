---
layout: page
title: Object Model
parent: Design and Internals
---

# Object Model

The runtime separates type/class metadata from instance state.

```text
Class
├── name
├── field definitions/defaults
├── constructor (`init`)
└── methods

Object
├── class reference
└── field values
```

## Why methods live on `Class`

An object should not duplicate the same method table for every instance. A class owns the function references and object instances refer back to that class.

## Bound methods

A `BoundMethod` contains the receiver and method name. The function is looked up when the bound method is called. The receiver is injected as `self` through the common argument-binding path.

## Struct and class syntax

`struct` and `class` currently share the runtime class/object model. Their distinction is primarily semantic and syntactic: structs encourage data-oriented declarations while classes make behavior-oriented object definitions explicit.

## Field defaults

Field defaults belong to the class definition but are evaluated for each object instance. This makes expressions such as the following instance-specific:

```novum
class Example {
    value = make_value()
}
```

The default is evaluated during instance construction rather than class declaration.

