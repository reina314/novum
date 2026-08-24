---
layout: page
title: Error and Control Flow Internals
parent: Design and Internals
---

# Error and Control Flow Internals

The interpreter separates ordinary returned values from control-flow signals.

```text
ControlFlow::Value(value)
ControlFlow::Return(value)
ControlFlow::Break
ControlFlow::Continue
```

This allows loops, function calls, blocks, and module execution to preserve non-local control flow without representing it as ordinary runtime values.

## Value contexts

`eval_value()` is stricter than generic `eval()`. If it encounters a `return`, `break`, or `continue` where an ordinary value is required, it reports a context-specific control-flow error rather than silently treating the control flow as a value.

Special cases such as `return Result.Err(...) ?` require call boundaries to preserve the underlying propagation semantics correctly. Tests should cover return/try interactions whenever control-flow handling changes.

## Error categories

The implementation distinguishes errors such as `Name`, `Type`, `Arity`, `Index`, `Import`, `Runtime`, `Control`, and overflow/range-related failures.

