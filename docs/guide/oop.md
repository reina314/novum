---
layout: page
title: Object-Oriented Programming
parent: Practical Guides
---

# Object-Oriented Programming

Novum's object model is deliberately compact. A `class`/`struct` declaration is compiled by the interpreter into a runtime `Class`; an instance is an `Object` that points to its class and owns its field values.

## Example

```novum
class Counter {
    value = 0

    increment = |self| {
        self.value += 1
    }
}

let c = Counter()
c.increment()
c.increment()
```

## Encapsulation

The current object model exposes fields through normal field access. More sophisticated visibility/mutability controls are possible future extensions.

## Constructors

Use `init` when construction requires behavior:

```novum
class Account {
    balance = 0

    init = |self, initial| {
        self.balance = initial
    }
}
```

The field default is established before `init` executes, so `init` can safely mutate it.

## Named constructor arguments

```novum
Account(initial=100)
```

Named arguments are resolved against the `init` parameter names.

