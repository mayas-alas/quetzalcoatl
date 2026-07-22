# MVP-CLOSE - Release candidate and focused completion

## Objective

Produce one byte-reproducible Quetzalcoatl 0.1.3 candidate with fixed release identity, simplify the repository to one documentation authority per topic, and retain one operational path to validate `controller + member-1` through the PVE join without expanding the three-node GNX MVP.

## Required work

- Assign 0.1.3 one explicit MSI ProductCode while preserving the existing UpgradeCode.
- Preserve the reviewed reboot contract: `PrepareWsl` maps 14 and 3010 to `forceReboot`; `ValidateHost` maps only 14; all other codes fail through the catch-all.
- Preserve the static-CRT, member join, secret transport, and controller-only writer contracts.
- Replace historical prompts and fragmented completed tasks with this single active task.
- Retain architecture, scope, decisions, tracker, evidence, and validation as the only authorities for their respective topics.
- Move any still-valid legacy statement into an authority before deleting its pointer document.
- Build the complete installer and record exact MSI/EXE hashes, sizes, version, and ProductCode.
- Under the explicitly authorized packaging-toolchain expansion, make the WiX bind deterministic by fixing PackageCode, summary/compound-file timestamps and the Burn registration ID; prove it with two byte-identical builds.
- Keep live validation honest: Dockur proves installer/runtime compatibility; the physical three-host gate alone proves quorum and G5.

## Operational finish

1. Publish the exact frozen 0.1.3 candidate to a temporary prerelease only after explicit remote authorization.
2. Run a clean `controller` slot and a clean `member-1` slot with the same hash.
3. Enter secrets only through the interactive noVNC/CLI path.
4. Prove reboot/resume, persisted roles, `PVE_JOIN=ready`, member `READY`, member OpenTofu denial, and no duplicate controller workloads.
5. Record run URLs and redacted evidence without claiming the physical three-host gate.

## Non-goals

- No fourth node, HA, promotion, concurrent first installs, new runtime, new backend, telemetry, or UI.
- No GitHub publish, workflow dispatch, commit, or push without explicit authorization.
- No secret values in source, arguments, logs, evidence, or reports.

## File ownership

- `Cargo.lock`
- `crates/gnx-cli/Cargo.toml`
- `crates/gnx-protocol/Cargo.toml`
- `crates/gnx-service/Cargo.toml`
- `crates/host-preflight/Cargo.toml`
- `installer/package.wxs`
- `installer/bundle.wxs`
- `installer/build.ps1`
- `installer/wixext/Gnx.DeterministicBundle.wixext/*`
- `AGENTS.md`
- `.AGENTS/README.md`
- `.AGENTS/SCOPE.md`
- `.AGENTS/DECISIONS.md`
- `.AGENTS/TRACKER.md`
- `.AGENTS/EVIDENCE.md`
- `.AGENTS/tasks/*.md`
- `.AGENTS/prompts/*.md`
- `docs/README.md`
- `docs/ARCHITECTURE.md`
- `docs/VALIDATION.md`
- `docs/TRACKING.md`
- `PoC.md`
- `quetzalcoatl-cierre-poc-mvp.md`

## Verification

```powershell
cargo fmt --all -- --check
cargo clippy -p gnx-service -- -D warnings
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1
git diff --check
```

Also verify Markdown references, the final Markdown inventory, embedded bundle payload hashes, MSI `ProductVersion`/`ProductCode`/`UpgradeCode`, and the exact reboot-contract negative cases.
