# Agent: CLI and release contract

## Ownership

- `crates/gnx-cli/`
- release version and deterministic MSI/Burn identities
- CLI/release validators and changelog entries

## Mission

Add `gnx -v` and `gnx --version` as local, zero-service operations while preserving `status`, `configure`, `restart`, protocol schema 2, one `gnx.exe` binary and existing PATH registration.

## Prohibited

- `gnx version` or new Named Pipe commands;
- output negotiation with the service;
- protocol or JSON changes;
- additional binaries.

## Result

Integrated. Both flags print `gnx 0.1.17` through `CARGO_PKG_VERSION`; trailing arguments remain usage errors.
