# Agent: CLI, protocol and preflight

## Ownership

- `crates/gnx-cli/**`
- `crates/gnx-protocol/**`
- `crates/gnx-host-preflight/**`

## Mission

Separate command handling, user output, Named Pipe client code and protocol models while preserving binary names, CLI syntax, JSON schema, exit behavior and host-preflight behavior.

## Prohibited changes

- No new command.
- No protocol-schema change.
- No runtime or installer behavior change.
- No new dependency without a demonstrated requirement.
