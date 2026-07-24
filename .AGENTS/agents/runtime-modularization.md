# Agent: runtime modularization

## Ownership

- `crates/gnx-service/src/runtime/`
- `runtime/payload/bin/gnx-runtime-agent`
- runtime portions of `docs/ARCHITECTURE.md`

## Objective

Reduce the Rust runtime monolith while preserving execution order, error codes, state transitions, machine generation and controller/member behavior.

## Guardrails

- Do not add a crate, daemon, listener or transport.
- Use the current Podman Machine SSH process transport.
- Keep pre-payload bootstrap operations direct; use the Fedora agent only after payload installation and handshake.
- Route only existing fixed PVE/Tailscale scripts through the agent.
- Do not alter state schema or runtime generation in 0.1.11.

## Handoff

Provide the release-integrity role with the exact payload paths, protocol marker and any changed build references. Do not mark complete until runtime validation passes.
