# Novum Documentation

This directory is intended to be served as a GitHub Pages/Jekyll site.

## Structure

```text
docs/
├── _config.yml
├── index.md
├── getting-started.md
├── language/
│   ├── index.md
│   ├── syntax.md
│   ├── values.md
│   ├── variables.md
│   ├── functions.md
│   ├── control-flow.md
│   ├── collections.md
│   ├── iterators.md
│   ├── ranges.md
│   ├── classes.md
│   ├── patterns.md
│   ├── modules.md
│   ├── errors.md
│   └── operators.md
├── reference/
│   ├── index.md
│   ├── grammar.md
│   ├── operators.md
│   ├── callables.md
│   ├── modules.md
│   └── types.md
├── std/
│   ├── index.md
│   ├── builtins.md
│   ├── math.md
│   ├── fs.md
│   ├── process.md
│   └── data.md
├── guide/
│   ├── index.md
│   ├── pipelines.md
│   ├── filesystem.md
│   ├── modules.md
│   ├── research.md
│   └── oop.md
└── internals/
    ├── index.md
    ├── runtime.md
    ├── modules.md
    ├── iterators.md
    ├── objects.md
    └── errors.md
```

The documentation is deliberately split by semantic area so that changes to one part of the language do not require editing a single monolithic reference page.

## Maintainability rules

- Keep user-facing syntax in `language/`.
- Keep callable/API behavior in `reference/`.
- Keep standard-library functions in `std/`.
- Keep practical usage examples in `guide/`.
- Keep implementation architecture in `internals/`.
- When an API changes, update the smallest relevant page first and cross-reference it from higher-level pages.
- Avoid documenting implementation details as language guarantees unless the behavior is intentional and tested.

