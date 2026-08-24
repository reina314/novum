---
layout: page
title: Standard Library
---

# Standard Library

The standard library is split conceptually into two layers:

1. **Eager builtins** installed in the interpreter environment.
2. **Lazy modules** loaded by `import` when needed.

The exact module catalog is still evolving. The sections below document APIs established during the current implementation line.

- [Builtins](builtins.md)
- [Math](math.md)
- [Filesystem (`fs`)](fs.md)
- [Process and Environment (`process`)](process.md)
- [Data and JSON](data.md)

