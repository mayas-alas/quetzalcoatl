# Controlled remote execution — normative policy

This document is the source of truth for commands that cross from the Windows service into the managed Fedora Podman Machine or a container running inside it.

## Why the boundary is strict

The execution path has several interpreters and filesystems:

```text
Windows service
→ podman.exe machine ssh
→ Fedora login shell / remote process
→ podman exec
→ managed container process
```

A quote, redirection or pipeline can be interpreted by a different layer than the author intended. The 0.1.17 Serve failure demonstrated this: `< /config/serve.json` was evaluated by Fedora before `podman exec` entered `gnx-tailscaled`.

The repository therefore uses one rule:

```text
argv describes the operation
stdin transports variable data
files represent durable state
shell syntax is not an operation API
```

## Current 0.1.17 primitives

The current transport exposes `machine_stdin` and `machine_stdin_output`. Their names are historical; the contract depends on how they are called.

### Fixed command without input

Use a fixed argv and an empty input:

```rust
machine_stdin(podman, ["true"], &[])?;
```

Required properties:

- every executable and subcommand is selected by repository code;
- variable values are validated arguments, not fragments of a command string;
- stdin is explicitly empty;
- no shell metacharacter is present in argv.

### Fixed command with data over stdin

Use stdin for JSON, configuration payloads, credentials and other variable data:

```rust
machine_stdin(
    podman,
    [
        "podman",
        "exec",
        "-i",
        "gnx-tailscaled",
        "tailscale",
        "serve",
        "set-raw",
    ],
    &serve_json,
)?;
```

The called program owns parsing of the input. The transport enforces input, output and timeout limits. Rust wraps copied input in `Zeroizing`; sensitive callers must still avoid logging or persisting the payload.

### Repository-owned multiline program

A multiline program is data sent to an interpreter that reads stdin:

```rust
machine_stdin(podman, ["sh", "-s"], SCRIPT.as_bytes())?;
machine_stdin(podman, ["python3", "-"], PROGRAM.as_bytes())?;
```

Allowed interpreters and modes are fixed by repository code. The script must be static or constructed only by a dedicated serializer with explicit validation. Runtime values should normally enter the program through its own bounded input format, not through string interpolation.

### Durable file

Use a file only when at least one of these is true:

- the value must survive service or machine restart;
- a daemon continuously consumes the path;
- later operations need the same artifact;
- an auditable versioned payload is required;
- the consumer has no stdin interface.

A durable write must follow:

```text
bounded content
→ fixed GNX-owned destination
→ write temporary sibling
→ flush / fsync where supported
→ validate size and optional digest
→ atomic rename
→ fixed permissions
```

A file path must be passed as a normal fixed or validated argument. Do not recreate shell redirection around it.

## Decision matrix

| Need | Required mechanism |
|---|---|
| Query or action without data | fixed argv, empty stdin |
| JSON or configuration accepted by a CLI | fixed argv, bounded stdin |
| Password, token or join material | fixed argv, bounded stdin, zeroization and redaction |
| Repository-owned shell program | `sh -s` plus script on stdin |
| Repository-owned Python probe | `python3 -` plus program on stdin |
| Persistent service configuration | atomic GNX-owned file, then fixed path argument |
| User-provided command text | prohibited |
| `sh -c` / `bash -c` | prohibited |
| `<`, `>`, `|`, `&&`, `||`, command substitution in argv | prohibited |

## Typed runtime-agent boundary

Operations executed by `gnx-runtime-agent` must be selected through `RuntimeOperation`. Each variant maps to one fixed argv shape in `runtime/remote/operation.rs`. Call sites must not append arbitrary arguments.

The Fedora agent must:

- recognize only allowlisted operations;
- validate exact argument counts and formats;
- reject trailing input where the operation has no stdin contract;
- avoid listeners and generic `exec` behavior;
- return bounded diagnostics without secrets.

## Direct machine and container operations

Not every machine operation currently passes through `RuntimeOperation`. Direct calls remain allowed only when all of the following are true:

1. the executable and subcommands are repository-selected;
2. argv contains no shell syntax;
3. every variable argument is validated for its semantic type;
4. variable payload data uses stdin;
5. a test asserts the exact command shape or resulting contract;
6. `ci/validate_remote_execution.py` can inspect the pattern.

New direct operations should prefer a small typed constructor rather than duplicating an argv array across call sites.

## Secrets and sensitive input

Sensitive values must not appear in:

- process argv;
- service status text;
- command error strings;
- debug output;
- temporary files;
- persisted runtime state unless explicitly protected by DPAPI.

The input must be size-limited, held for the shortest practical lifetime and zeroized after the child process completes. A failed operation may report the operation name and exit status, but not its input.

## Forbidden examples

```rust
machine_stdin(
    podman,
    ["sh", "-c", "tailscale serve set-raw < /config/serve.json"],
    &[],
)?;
```

```rust
let command = format!("pvecm add {controller} --password {password}");
```

```rust
machine_stdin(podman, caller_supplied_argv, caller_supplied_input)?;
```

These patterns fail because they move parsing, quoting or authorization outside the typed repository contract.

## Future API normalization

A later release may rename the transport into explicit primitives:

```text
machine_exec
machine_exec_with_stdin
machine_run_script
machine_write_file
```

Those names are a migration target, not APIs implemented by 0.1.17. New 0.1.17 changes must follow the behavior in this policy using the existing transport.

## Enforcement

`ci/validate_remote_execution.py` checks:

- fixed `RuntimeOperation` mappings;
- absence of free-form runtime-agent argv;
- input, output, timeout and cancellation guards;
- absence of `sh` or `bash` followed by `-c`;
- absence of shell-control operators in direct remote argv arrays;
- runtime-agent hardening markers and shell syntax where a POSIX shell is available.

Static validation does not prove Windows, Podman Machine or container behavior. The Windows build and acceptance sequence in `VALIDATION.md` remains required.
