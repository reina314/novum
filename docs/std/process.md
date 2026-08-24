---
layout: page
title: Process and Environment
parent: Standard Library
---

# Process and Environment (`process`)

The `process` module groups command-line, environment, working-directory, and child-process operations.

```novum
import process
```

## `process.args()`

Returns the command-line arguments as a list of strings.

```novum
let args = process.args()
```

## `process.env(name)`

Reads an environment variable and returns an `Option`:

```novum
match process.env("HOME") {
    Option.Some(value) => print(value)
    Option.None => print("not set")
}
```

## `process.cwd()`

Returns the current working directory as a `Result<Path, ...>` style value in the newer API:

```novum
let cwd = process.cwd()?
let data = cwd.join("data")
```

## `process.set_env(name, value)`

Set an environment variable and return a fallible result.

```novum
process.set_env("NOVUM_MODE", "dev")?
```

## `process.run(command, args)`

Run a process directly, without invoking a shell:

```novum
let result =
    process.run(
        "git",
        ["status"]
    )?

print(result.stdout)
print(result.stderr)
print(result.status)
```

The conceptual result object contains:

```text
status
stdout
stderr
```

A process that starts successfully but exits with a non-zero status is still represented as a successful process execution result; inspect `status` rather than treating every non-zero exit code as a host-level execution failure.

This distinction allows programs to decide whether a command's exit status is acceptable.

