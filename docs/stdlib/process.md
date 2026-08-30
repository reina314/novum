---
layout: default
title: Process and Environment
parent: Standard Library
nav_order: 6
---

# `process`

Import with:

```novum
import process
```

The process module exposes command-line arguments, environment variables, the current working directory, and child-process execution.

## `process.args()`

Returns a `List[Str]` containing the command-line arguments passed to the Novum program **excluding the executable name**.

```novum
for arg in process.args() {
    print(arg)
}
```

## `process.env(name)`

Reads an environment variable.

```novum
let home = process.env("HOME")
```

The result is an `Option`: `Option.Some(Str)` when the variable exists and `Option.None` when it does not.

## `process.set_env(name, value)`

Sets an environment variable for the current process and returns `Result.Ok(Unit)` on success.

```novum
process.set_env("NOVUM_MODE", "debug")
```

## `process.cwd()`

Returns the current working directory as a `Path`, wrapped in `Result`:

```novum
let cwd = process.cwd()
```

The successful result is `Result.Ok(Path)`.

## `process.run(command, arguments)`

Runs an external command and captures its output.

```novum
let result =
    process.run(
        "python",
        ["--version"]
    )
```

The second argument must be a `List[Str]`.

On success, the returned value is `Result.Ok(Dict)` with these properties:

| Property | Type | Meaning |
|---|---|---|
| `status` | `Int` | Process exit code, or `-1` when no ordinary code is available |
| `stdout` | `Str` | Captured standard output |
| `stderr` | `Str` | Captured standard error |

Example:

```novum
let result = process.run("echo", ["hello"])

match result {
    Result.Ok(output) => {
        print(output["status"])
        print(output["stdout"])
    }
    Result.Err(message) => print(message)
}
```

{% callout warning %}
`process.run()` executes a real external process. Never pass untrusted command names or arguments to it without validating them first.
{% endcallout %}
