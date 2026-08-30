---
layout: default
title: JSON
parent: Standard Library
nav_order: 4
---

# `json`

Import with:

```novum
import json
```

## `json.parse(text)`

Parses a JSON string into ordinary Novum values.

```novum
let value = json.parse("{\"name\": \"Novum\", \"count\": 3}")

print(value["name"])
print(value["count"])
```

JSON values map to Novum as follows:

| JSON | Novum |
|---|---|
| `null` | `Null` |
| boolean | `Bool` |
| integer | `Int` when representable |
| non-integer number | `Float` |
| string | `Str` |
| array | `List` |
| object | `Dict` |

JSON integers outside Novum's `Int` range are rejected rather than silently truncated.

## `json.stringify(value)`

Serializes a compatible Novum value to a compact JSON string:

```novum
let data = {
    "name": "Novum",
    "items": [1, 2, 3]
}

let text = json.stringify(data)
print(text)
```

The following Novum values can be serialized directly: `Null`, `Unit` (as JSON `null`), `Bool`, `Int`, `Float`, `Str`, `List`, and `Dict`.

Values such as classes, functions, iterators, matrices, and paths are not automatically JSON-serializable.

Non-finite floating-point values that cannot be represented as JSON numbers are rejected.
