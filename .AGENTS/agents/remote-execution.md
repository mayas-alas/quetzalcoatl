# Agent: remote execution

## Ownership

- `crates/gnx-service/src/runtime/remote/`
- direct `machine_stdin` / `machine_stdin_output` call sites
- `runtime/payload/bin/gnx-runtime-agent`
- `ci/validate_remote_execution.py`
- `docs/REMOTE_EXECUTION.md`
- `docs/REMOTE_EXECUTION_REVIEW.md`

## Mission

Keep every cross-machine operation inside the argv/stdin/file contract:

- argv identifies a closed operation;
- stdin carries bounded variable data, JSON, scripts and secrets;
- files exist only for durable GNX-owned state or a consumer that requires a path.

## Prohibited

- `sh -c` or `bash -c`;
- redirection, pipelines, command substitution or chained commands in remote argv;
- caller-provided commands or arbitrary runtime-agent arguments;
- secrets in argv, logs, errors or unprotected temporary files;
- undocumented paths whose execution layer is ambiguous.

## Result

The transport enforces bounded stdin/output, timeout, cancellation and zeroization. The validator rejects shell-string execution and shell-control syntax in explicit remote argv arrays. Every new operation requires the review evidence in `docs/REMOTE_EXECUTION_REVIEW.md`.
