# Quetzalcoatl agent contract

## Mission

Maintain a minimal Windows bootstrap that converges WSL, a dedicated Fedora Podman Machine, required devices, Proxmox VE, Tailscale and a resumable Proxmox cluster role.

## Required reading

1. `.AGENTS/SCOPE.md`
2. `.AGENTS/DECISIONS.md`
3. `.AGENTS/TRACKER.md`
4. `docs/ARCHITECTURE.md`
5. `.AGENTS/tasks/PROXMOX_CLUSTER_RUNTIME.md`

## Engineering rules

- Keep controller/member role selection persistent and immutable after first resolution.
- Keep member join resumable and pinned to the original controller.
- Never publish Proxmox ports on Windows.
- Move secrets only through DPAPI, stdin or ephemeral root-only files under `/run`.
- Keep the runtime surface limited to Proxmox, Tailscale and cluster convergence.
- Runtime payload files must be LF-only, hash-locked, immutable and represented exactly once in `manifest.json` and `PAYLOAD_FILES`.
- Do not claim physical cluster or Windows installer validation without captured evidence.

## Verification

```powershell
python ci/validate_runtime.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
