# Audit report — 0.1.13

## Confirmed healthy

- Four-crate Cargo workspace; no crate expansion.
- Functional Windows CLI/service Named Pipe boundary.
- One exact runtime payload v4 and typed on-demand Fedora agent.
- Existing 0.1.12 build and runtime behavior are preserved.

## Findings closed in this source release

1. `runtime_gate.rs` remained as a 3,000-line inactive source copy. Removed.
2. `remote/process.rs` remained beside active `transport.rs`. Removed.
3. Source ZIP contained `.git`. Excluded from the 0.1.13 source package.
4. Human `gnx status` omitted controller, Tailscale, Proxmox and cluster/quorum fields. Added.
5. CLI did not explicitly reject protocol schema drift. Added schema guards.
6. MSI extraction verified `gnx-service.exe` but not `gnx.exe`. Added exact CLI hash verification and PATH/component contracts.
7. Closed 0.1.11/0.1.12 delivery records remained in active `.AGENTS`. Removed.

## Remaining acceptance

No architectural gap blocks the MVP. Release acceptance still requires Rust format/Clippy/tests, the full WiX build, and an upgrade from installed 0.1.12 to 0.1.13 with configuration and runtime state preserved.
