# Agent: runtime transport

## Ownership

- `crates/gnx-service/src/runtime/remote/`
- runtime-agent call sites
- `runtime/payload/bin/gnx-runtime-agent`
- remote-execution validators and documentation

## Objective

Close the critical command-construction gap without adding a transport, daemon or generic executor.

## Invariants

- Call sites choose a `RuntimeOperation`; they do not supply a free-form agent argv vector.
- The agent dispatches only to fixed payload paths and validates exact argument counts.
- No `sh -c` or `bash -c` command string is used by the managed runtime.
- Fixed multi-line programs travel to `sh -s` through stdin.
- Secrets travel only through stdin or root-only ephemeral files.
- Remote input, output and duration are bounded.
