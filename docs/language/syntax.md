---
layout: page
title: Syntax Overview
parent: Language Guide
---

# Syntax Overview

Novum uses a compact expression-oriented syntax.

## Statements and expressions

Most constructs are expressions. Blocks evaluate their contained expressions and produce the value of the last successful expression.

```novum
{
    x = 10
    y = 20
    x + y
}
```

## Comments

The exact comment forms are lexer-defined. Keep comments consistent with the syntax accepted by the current lexer.

## Identifiers

Identifiers are used for variables, functions, fields, modules, types, and parameters.

## Literals

Common literals include:

```novum
42
3.14
true
false
"hello"
[1, 2, 3]
```

## Calls

```novum
f(1, 2)
obj.method(1)
```

Named arguments are supported for user-defined functions, methods, and struct/class constructors:

```novum
f(y=3, x=2)
Point(x=2, y=3)
```

Positional arguments must precede named arguments:

```novum
f(1, y=2)     // valid
f(x=1, 2)     // invalid
```

## Access

Field access:

```novum
object.field
```

Indexing:

```novum
xs[0]
d["key"]
```

Tuple indexing uses dot syntax where supported by the parser:

```novum
pair.0
```

## Assignment

Simple assignment:

```novum
x = 10
```

Compound assignment:

```novum
x += 1
xs[0] += 1
object.value *= 2
```

## Import aliases

```novum
import math as m
m.sqrt(16)
```

