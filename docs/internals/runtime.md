---
layout: page
title: Runtime Architecture
parent: Design and Internals
---

# Runtime Architecture

Novum is implemented as an interpreter with a pipeline broadly resembling:

```text
source
  ↓
Lexer
  ↓
Parser
  ↓
Program / Expr AST
  ↓
Interpreter
  ↓
Runtime Value
```

## Environment

The environment is a chain of frames. A child environment can be created for lexical/block scope.

The current environment API includes operations conceptually equivalent to:

```text
global()
new(parent)
child()
define(name, value)
get(name)
assign(name, value)
local_values()
contains_local(name)
remove_local(name)
```

## Control flow

Evaluation returns a `ControlFlow` value with cases such as:

```text
Value
Return
Break
Continue
```

`eval_value()` converts inappropriate control flow into context-specific errors rather than silently discarding it.

## Call arguments

User-visible call syntax is represented as call arguments containing:

```text
name: Option<String>
value: Expr
```

After evaluation they become evaluated arguments. A shared binder maps them to lambda/class parameter names.

## Runtime objects

Objects contain fields and reference their `Class`. Methods live on the class rather than being duplicated into every object instance.

