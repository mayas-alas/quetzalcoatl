# Quetzalcoatl agent contract — 0.1.12

## Mission

Maintain the smallest Windows bootstrap that converges WSL2, one dedicated Fedora Podman Machine, Proxmox VE, Tailscale and a resumable cluster role without expanding the MVP boundary.

## Required reading

1. `.AGENTS/SCOPE.md`
2. `.AGENTS/DECISIONS.md`
3. `.AGENTS/TRACKER.md`
4. `.AGENTS/tasks/RELEASE_0.1.12.md`
5. `.AGENTS/tasks/REMOTE_EXECUTION_REMEDIATION.md`
6. `.AGENTS/tasks/RECONCILER_RECOVERY.md`
7. `docs/ARCHITECTURE.md`
8. `docs/REMOTE_EXECUTION.md`

## Delivery roles

- Runtime transport: typed Fedora operations and bounded process execution.
- Reconciler recovery: convergence order, persistence and resume invariants.
- Release integrity: payload, installer, identities and cross-role validation.

A change is complete only after all affected roles agree that it remains in scope and the applicable validation commands pass.

## Engineering rules

- Keep exactly the four existing Rust crates.
- Preserve CLI, Named Pipe, state schema and machine generation contracts.
- Keep controller/member role selection persistent after first resolution.
- Never publish Proxmox ports on Windows.
- Move secrets only through DPAPI, stdin or ephemeral root-only files under `/run/gnx`.
- Invoke the Fedora agent on demand through Podman Machine SSH; it must not listen or expose arbitrary exec.
- Select agent work through `RuntimeOperation`, never through a call-site-provided argv array.
- Never use `sh -c` or `bash -c` for managed runtime execution.
- Use stdin-fed `sh -s` only for fixed bootstrap/probe programs without external interpolation.
- Keep exactly one source payload at `runtime/payload`; files must be LF-only, hash-locked and allowlisted once.
- Keep `installer/build.ps1` as the only release-build entry point.
- Do not add GitHub Actions, OpenTofu, tray UI, a new service or a new crate in 0.1.12.
- Do not claim build, upgrade or physical cluster acceptance without captured evidence.

## Verification

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_release_contract.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```
