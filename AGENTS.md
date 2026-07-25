# Quetzalcoatl agent contract — 0.1.13

## Mission

Maintain the smallest Windows bootstrap that converges WSL2, one dedicated Fedora Podman Machine, Proxmox VE, Tailscale and a resumable cluster role without expanding the MVP boundary.

## Required reading

1. `.AGENTS/SCOPE.md`
2. `.AGENTS/DECISIONS.md`
3. `.AGENTS/TRACKER.md`
4. `.AGENTS/tasks/RELEASE_0.1.13.md`
5. `.AGENTS/tasks/CLI_CONTRACT_AUDIT.md`
6. `.AGENTS/tasks/REMOTE_EXECUTION_REMEDIATION.md`
7. `.AGENTS/tasks/RECONCILER_RECOVERY.md`
8. `docs/ARCHITECTURE.md`
9. `docs/AUDIT_0.1.13.md`

## Delivery roles

- CLI contract: command surface, output, protocol compatibility and MSI installation.
- Runtime transport: typed Fedora operations and bounded process execution.
- Reconciler recovery: convergence order, persistence and resume invariants.
- Release integrity: payload, installer, identities and cross-role validation.

## Engineering rules

- Keep exactly the four existing Rust crates.
- Preserve the CLI command set, Named Pipe schema, state schema and machine generation.
- `gnx status` human and JSON output must expose the complete MVP status contract.
- The MSI must install exactly one freshly built `gnx.exe` and register its directory in the system PATH.
- Keep controller/member role selection persistent after first resolution.
- Never publish Proxmox ports on Windows.
- Move secrets only through DPAPI, stdin or ephemeral root-only files under `/run/gnx`.
- Invoke the Fedora agent on demand through Podman Machine SSH; it must not listen or expose arbitrary exec.
- Never use `sh -c` or `bash -c` for managed runtime execution.
- Keep exactly one source payload at `runtime/payload`; files must be LF-only, hash-locked and allowlisted once.
- Keep `installer/build.ps1` as the only release-build entry point.
- Do not add GitHub Actions, OpenTofu, tray UI, a new service or a new crate in 0.1.13.
- Do not retain inactive source copies inside compiled module trees.

## Verification

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1 -TestRebootContractOnly
.\installer\build.ps1
```
