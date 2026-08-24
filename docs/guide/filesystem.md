---
layout: page
title: Filesystem and Paths
parent: Practical Guides
---

# Filesystem and Paths

Use `Path` rather than manual string concatenation when composing filesystem paths.

```novum
import process
import fs

let root = process.cwd()?
let config = root.join("config.json")

let text = fs.read(config)?
```

Inspect path components:

```novum
config.name()
config.stem()
config.extension()
config.parent()
```

Check filesystem state:

```novum
if config.exists() {
    print(fs.read(config)?)
}
```

Convert a path to a string only at API boundaries that specifically require text:

```novum
config.to_str()
```

