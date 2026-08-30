---
layout: default
title: Filesystem
parent: Standard Library
nav_order: 5
---

# `fs`

Import with:

```novum
import fs
```

Filesystem functions accept either a `Str` path or a `Path` value produced by `path(...)`.

## `fs.read(path)`

Reads a UTF-8 text file.

```novum
let result = fs.read("notes.txt")
```

On success the function returns a `Result.Ok(Str)`. On failure it returns `Result.Err(message)`.

## `fs.write(path, content)`

Writes text to a file, replacing its previous contents.

```novum
fs.write("out.txt", "hello\n")
```

Returns `Result.Ok(Unit)` on success or `Result.Err(message)` on failure.

## `fs.append(path, content)`

Appends text to a file. The file is created when it does not already exist.

```novum
fs.append("log.txt", "next line\n")
```

## `fs.exists(path)`

Returns a `Bool` indicating whether the path exists.

```novum
if fs.exists("data.csv") {
    print("found")
}
```

## `fs.remove(path)`

Removes a file and returns `Result.Ok(Unit)` or `Result.Err(message)`.

```novum
fs.remove("temporary.txt")
```

## `fs.mkdir(path)`

Creates a directory and any missing parent directories.

```novum
fs.mkdir("results/2026")
```

## `fs.rename(from, to)`

Renames or moves a filesystem entry using the platform's filesystem semantics.

```novum
fs.rename("draft.txt", "final.txt")
```

## `fs.copy(from, to)`

Copies a file. On success the result is `Result.Ok(Int)` where the integer is the number of bytes copied.

```novum
let result = fs.copy("data.csv", "backup/data.csv")
```

## `fs.list_dir(path)`

Lists the entries of a directory.

```novum
let entries = fs.list_dir("data")
```

On success the result is `Result.Ok(List[Str])`, containing entry names rather than full paths.

> **Warning**
>
> The filesystem module performs real I/O. Paths and contents are interpreted by the operating system running Novum.
