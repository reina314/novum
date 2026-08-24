---
layout: page
title: Filesystem (fs)
parent: Standard Library
---

# Filesystem (`fs`)

The filesystem module provides fallible file and directory operations. APIs return `Result` values so callers can use `?` propagation.

Typical import:

```novum
import fs
```

## Paths

Filesystem functions accept either strings or `Path` values where the API supports both:

```novum
fs.read("data.txt")?
fs.read(path("data.txt"))?
```

## `read(path)`

Read a UTF-8 text file.

```novum
let text = fs.read(path("data.txt"))?
```

## `write(path, content)`

Write text to a file.

```novum
fs.write(
    path("output.txt"),
    "hello"
)?
```

## `append(path, content)`

Append text to a file, creating it when appropriate.

```novum
fs.append(
    path("log.txt"),
    "entry\n"
)?
```

## Other filesystem operations

The filesystem layer also contains operations for directory creation, listing, copy, rename, removal, and existence/type checks as they are exposed by the current implementation.

Errors should be propagated or handled explicitly:

```novum
match fs.read(path("data.txt")) {
    Result.Ok(text) => print(text)
    Result.Err(message) => print(message)
}
```

## Path formatting in host implementation

Rust `Path`/`PathBuf` values should be formatted with `.display()` in diagnostic messages. Novum user-facing `Path` values are represented by `PathValue` and can be converted with `to_str()`.

