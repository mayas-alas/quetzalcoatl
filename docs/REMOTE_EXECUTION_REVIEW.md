# Remote-execution review checklist

Use this checklist for every change that starts a process in Windows, enters Podman Machine, invokes `podman exec`, runs a payload script or sends input to `gnx-runtime-agent`.

## 1. Classify the operation

Record one category:

- fixed command without input;
- fixed command with structured stdin;
- repository-owned script through interpreter stdin mode;
- durable atomic file;
- typed runtime-agent operation.

A change that does not fit one category must stop for architecture review.

## 2. Prove the command boundary

- [ ] The executable and subcommands are selected by repository code.
- [ ] The complete argv shape is visible in one typed constructor or one narrow call site.
- [ ] No `sh -c`, `bash -c`, redirection, pipeline, command substitution or chained command exists.
- [ ] Variable arguments are validated semantic values, not shell fragments.
- [ ] Caller-provided command text cannot reach the transport.

## 3. Prove the data boundary

- [ ] JSON, configuration and secrets use stdin when the consumer supports it.
- [ ] Input has a documented maximum size.
- [ ] Sensitive input is zeroized and never included in an error.
- [ ] The child process receives EOF after the complete input.
- [ ] Empty-input operations pass an explicit empty slice.

## 4. Prove the file boundary

Complete this section only when a path is required:

- [ ] The consumer cannot reasonably accept stdin, or the value must be durable.
- [ ] The destination is fixed and GNX-owned.
- [ ] Writes use a temporary sibling and atomic rename.
- [ ] Size, optional digest and permissions are validated.
- [ ] No shell redirection is used to read or write the file.
- [ ] Restart and partial-write behavior is tested.

## 5. Prove the layer

Document where each item exists:

| Item | Windows | Fedora machine | Container |
|---|---:|---:|---:|
| executable |  |  |  |
| input source |  |  |  |
| optional file |  |  |  |
| output parser |  |  |  |

A path such as `/config/serve.json` is invalid unless the review identifies the exact layer where it is mounted and the exact process that opens it.

## 6. Required tests

- [ ] Exact argv contract.
- [ ] Expected stdin policy and representative payload.
- [ ] Rejection of malformed or oversized input.
- [ ] Timeout and non-zero exit mapping.
- [ ] Output limit and redaction behavior.
- [ ] Restart or retry behavior where the operation mutates state.
- [ ] `python .\ci\validate_remote_execution.py` passes.

## 7. Pull-request evidence template

```text
Operation:
Category:
Executable layer:
Argv owner:
Stdin schema and maximum:
Sensitive data:
Durable files:
Timeout:
Expected idempotency:
Tests:
Validator impact:
```

The default exception set is empty. An exception must be recorded in `.AGENTS/DECISIONS.md`, narrowly represented in the validator and assigned an explicit removal condition.
