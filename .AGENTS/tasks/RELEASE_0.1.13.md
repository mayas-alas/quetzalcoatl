# Release 0.1.13 — CLI contract and source hygiene

## Goal

Finish the installed 0.1.12 MVP without changing runtime behavior: remove inactive sources, make the CLI status contract complete, and prove the exact CLI binary is installed.

## Required changes

1. Delete legacy `runtime_gate.rs` and `remote/process.rs`.
2. Remove closed 0.1.11/0.1.12 delivery records from active scope.
3. Preserve the three-command CLI surface.
4. Expose controller, Tailscale, Proxmox and cluster fields in human status.
5. Validate protocol schema versions in the CLI.
6. Verify MSI CLI component, PATH entry and extracted `gnx.exe` hash.
7. Generate new 0.1.13 release identities preserving upgrade families.

## Non-goals

No new command, crate, runtime stage, state migration, payload revision, service, listener or hosted pipeline.
