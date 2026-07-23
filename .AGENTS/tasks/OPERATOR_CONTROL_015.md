# OPERATOR-CONTROL 0.1.5 - Native recovery controls

## Objective

Produce Quetzalcoatl 0.1.5 as the next upgrade over the frozen 0.1.4
candidate. Add two operator controls without weakening the privileged-service
or secret-transport boundaries:

- `gnx restart` performs the Windows SCM equivalent of
  `Restart-Service -Name Quetzalcoatl` and requires an elevated local
  administrator.
- `gnx configure forgejo` interactively sets the managed Forgejo administrator
  username and password on a ready controller.

## Required behavior

- Forgejo credentials are read with console echo disabled and never enter
  process arguments, logs, state, Compose, or OpenTofu state.
- The CLI sends the credential only through the existing local named pipe.
- `gnx-service` remains the authority for Forgejo mutation and persists the
  managed credential with user-scope DPAPI.
- Rotation is resumable across a failure between the Forgejo API mutation and
  DPAPI commit.
- The Linux transport uses stdin and root-only ephemeral `/run` files.
- Members, controllers without Forgejo, and non-ready controllers reject the
  operation explicitly.
- Restart uses the Windows Service Control Manager, never PowerShell command
  construction, and reports timeout or SCM failures without changing product
  state.
- 0.1.5 receives new MSI, package, and Burn identities while preserving the
  existing upgrade codes.

## File ownership

- `Cargo.lock`
- `crates/gnx-cli/**`
- `crates/gnx-protocol/**`
- `crates/gnx-service/**`
- `crates/host-preflight/Cargo.toml`
- `runtime/payload-v1/**`
- `installer/package.wxs`
- `installer/bundle.wxs`
- `installer/build.ps1`
- `installer/wixext/Gnx.DeterministicBundle.wixext/**`
- `.AGENTS/DECISIONS.md`
- `.AGENTS/TRACKER.md`
- `.AGENTS/EVIDENCE.md`
- `.AGENTS/tasks/OPERATOR_CONTROL_015.md`
- `docs/ARCHITECTURE.md`
- `docs/VALIDATION.md`
- `installer/docs/RUNBOOK.md`

## Verification

```powershell
cargo fmt --all -- --check
cargo clippy -p gnx-service -- -D warnings
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1
git diff --check
```

The physical Forgejo rotation and SCM restart remain live-unverified until
captured on the Windows controller. Do not infer those results from static or
unit tests.
