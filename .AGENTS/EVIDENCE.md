# Evidence ledger

## Acceptance rule

An item is evidence only when another operator can identify the exact revision, environment, command or user action, expected result, actual result, and retained artifact. Never record secret values.

Use this shape:

```text
Evidence ID:
Gate:
Revision or artifact hash:
Environment:
Command or end-user action:
Expected:
Actual:
Artifact path or run URL:
Secret scan result:
Verdict: PASS | FAIL | INCONCLUSIVE
```

## Existing evidence retained from legacy tracking

The complete pre-normalization documents remain recoverable at revision `4c4cfd5bf5757480bdd19020bae10db6b2169c21`; they are intentionally absent from the maintained tree. Their integrity anchors are:

- `PoC.md`: SHA-256 `93C584A4FE5193CA1AD740C3D5DCA5D15A445F68659810758BCC1325BF56C9D7`.
- `docs/TRACKING.md`: SHA-256 `98A68F2A32E85C49CDF30FE87E4B07D5FAAE2B998041D34D47ABC1612E4A896B`.

Legacy evidence is useful regression context but awards no current-cycle points until reproduced against the integrated candidate.

### E-LEGACY-DOCKUR-01

- Gate: historical CI feasibility only; does not score G2 until reproduced.
- Environment: GitHub-hosted Linux runner with Dockur Windows guests.
- Actual: two guests became reachable through RDP, but Tailscale paths used DERP with approximately 64-73 ms RTT.
- Verdict: `INCONCLUSIVE` for compatibility regression and `FAIL` for the G5 Corosync network gate.
- Source: `docs/TRACKING.md` at revision `4c4cfd5bf5757480bdd19020bae10db6b2169c21`.

### E-LEGACY-INSTALLER-01

- Gate: historical installer and lifecycle baseline.
- Environment: elevated consumer Windows 11 x64 host.
- Actual: WiX validation, major upgrade, exact 30/30 payload comparison, and installed service/CLI checks passed for the prior controller candidate.
- Artifacts: setup SHA-256 `B219880578A75EA13C4800F60E2F46CC43AF4BD51DDD8C5DACA2A00F5A6DADE6`; MSI SHA-256 `6D97299F43B9D1D1E79B118D7236CE2705B63ACE71935E57AFE7E98343906792`; source commit `b2428da`.
- Verdict: `PASS` for the historical candidate; `INCONCLUSIVE` for the current member candidate until rebuilt and rerun.

### E-LEGACY-CONTROLLER-01

- Gate: historical controller functional baseline.
- Environment: installed Windows 11 service before and after a real reboot.
- Actual: `gnx status --json` returned `READY`; PVE was joined/quorate; OpenTofu was ready; Garage passed S3 PUT/GET; Forgejo passed push/clone; the same controller state returned after reboot in 267.7 seconds.
- Artifacts: service SHA-256 `3B10EE43FB90B92664FF9685C684485D6B7ABE9FAA401449EC98E2AD9C4780A9`; manifest SHA-256 `CAC7581621CF768CB8C92DAD90B3E94B2454BC991C5D086BED666A97CD1E7CE8`; source commits `59607d8` and `b2428da`.
- Verdict: `PASS` for the prior controller path; must be rerun after member integration.

### E-LEGACY-SECURITY-01

- Gate: historical controller exposure and secret baseline.
- Environment: installed Windows 11 controller after reboot.
- Actual: state plus two DPAPI blobs were restricted to SYSTEM and the service identity; PVE/Garage/Forgejo were reachable through approved tailnet endpoints; no Windows listeners existed on ports 22, 443, 2222, 3000, 3900, or 8006.
- Verdict: `PASS` for the prior controller candidate; current release still requires a new secret scan and listener inventory.

## New evidence

### E-BASELINE-01

- Gate: regression baseline; prerequisite for G3 and G4.
- Revision: `4c4cfd5bf5757480bdd19020bae10db6b2169c21` plus documentation-only uncommitted orchestration files.
- Environment: Windows development host, Rust toolchain pinned by `rust-toolchain.toml`.
- Command: `cargo fmt --all -- --check`; `cargo test --workspace`.
- Expected: clean formatting and all existing tests passing.
- Actual: format check passed; 21 tests passed, 0 failed.
- Artifact: current task terminal transcript.
- Secret scan: commands did not emit configured secret values.
- Verdict: `PASS`.

### E-GRAPH-01

- Gate: orchestration support; does not add closure points by itself.
- Environment: isolated Graphify 0.9.23 runtime under ignored `target/graphify-runtime`.
- Action: incremental AST and semantic update after adding the closure contract and `.AGENTS/` control plane.
- Actual: 544 nodes, 1,475 edges, 42 labeled communities; health check reported zero dangling endpoints, missing endpoints, self-loops, duplicates, or collapsed edges.
- Diff from prior graph: 485 nodes and 1,381 edges added; 25 nodes and 51 edges removed.
- Artifacts: `graphify-out/graph.json`, `graphify-out/graph.html`, `graphify-out/GRAPH_REPORT.md`.
- Token accounting: the native Graphify worker result did not expose usage metadata, so this run is recorded as zero rather than inventing a count.
- Verdict: `PASS` with the stated token-accounting limitation.

### E-CORE-01

- Gate: G3 partial evidence.
- Baseline revision: `4c4cfd5bf5757480bdd19020bae10db6b2169c21`.
- Environment: isolated `codex/core-state` worktree on Windows.
- Change: persisted `Member` role/identity/controller pin/join state plus final member-ready protocol representation and legacy controller deserialization.
- Verification: `cargo fmt --all -- --check`, affected crate tests, `cargo test --workspace`, and `git diff --check` all passed after architect-requested corrections.
- Architect review: exact `gnx-member-*` naming, bounded controller ID, `READY`, joined/quorate, local components ready, and controller-only workloads `not_applicable` confirmed.
- Secret scan: no secret-bearing fields added to persisted state or status.
- Verdict: `PASS`; G3 remains open until CORE-02 is integrated and tested.

### E-CORE-02

- Gate: G3 partial evidence; no points awarded until the PVE member stage is integrated into the runtime and committed.
- Baseline revision: `4c4cfd5bf5757480bdd19020bae10db6b2169c21`; integrated working-tree patch hash `b5fd7c84a8cee9ad9315d2a53a9f2011d78bfb0c`.
- Environment: primary Windows development checkout.
- Change: bounded 0/1/2-peer role matrix, exact peer filtering, one-time member persistence, pinned controller retries, role-aware Tailscale reauthentication, local member hostname/Serve handling, and controller-workload exclusion.
- Primary-source audit: Tailscale `v1.98.8` treats a non-empty `CurAddr` as the active direct path even when a DERP region is present; the parser and fixture were corrected against that source contract.
- Verification: `cargo fmt --all -- --check`, `cargo clippy -p gnx-service -- -D warnings`, `cargo test --workspace` (34 tests), and `git diff --check` passed in the primary checkout.
- Secret scan: no configuration values or secret-bearing fields were added to discovery, state, tests, or status.
- Verdict: `PASS` for CORE-02; runtime-to-PVE join integration and live member evidence remain open.

### E-PVE-01

- Gate: G4 code-level payload evidence; physical execution remains part of G5.
- Source revision: `06cbec0250bc574d5187c63a602d75a93b0ee1ea`; integrated code patch SHA-1 `c3b05f3d23a25baedce59a81445af3a5b2897309`.
- Environment: primary Windows checkout plus Git Bash POSIX shell.
- Change: fixed five-line stdin contract, direct Tailscale gate, API/SSH/Corosync/MTU/time/DNS preflights, certificate fingerprint pinning, transient `0600` password file under `/run`, idempotent joined-state verification, and no controller fallthrough.
- Review corrections: routed verbose `pvecm add` output away from the exact success `stdout`, rejected all trailing stdin, mapped absent pinned controller consistently, and updated the locked manifest hash.
- Verification: `sh -n`, payload `static-check`, manifest Rust test, manifest SHA comparison, and two independent `medium` Codex CLI reviews; the correction review returned `CLEAN`.
- Payload SHA-256: `6AEAE8126262A2157566FBC4E0BFC747325BE63CF51CDBD5CABAEDCDB8AEA7CB`.
- Secret scan: password is never an argument or persisted in state/status; Rust join input uses a zeroizing buffer and captured payload output is zeroized.
- Verdict: `PASS` for G4 implementation evidence; three-host behavior remains explicitly unclaimed.

### E-INT-01

- Gates: G3 and G4.
- Source revision: `06cbec0250bc574d5187c63a602d75a93b0ee1ea`; integrated code patch SHA-1 `c3b05f3d23a25baedce59a81445af3a5b2897309`.
- Environment: primary Windows development checkout, Rust 1.96.1.
- Change: persisted `NotStarted -> Joining -> Joined`, pinned-controller reboot verification, final member `READY` status, stable operational errors, and production `MEMBER_OPENTOFU_DENIED` guard before process launch.
- Verification: `cargo fmt --all -- --check`; `cargo clippy -p gnx-service -- -D warnings`; `cargo test --workspace` (40 tests: 3 protocol and 37 service); `git diff --check`.
- Review history: initial `medium` review found two P1 and two P2 stream/availability gaps; architect corrections were applied and a second focused `medium` review returned `CLEAN`.
- Secret scan: no secret-bearing persisted/status field; no password clone in join input construction; DPAPI configuration and child buffers are zeroized on all represented outcomes.
- Verdict: `PASS`; G3 and the code-level G4 gate are verified. G5 remains blocked on real hosts.

### E-PKG-01

- Gate: packaging prerequisite for G2/G5/G6; it does not award G6 by itself.
- Source revision: `06cbec0250bc574d5187c63a602d75a93b0ee1ea`; integrated code patch SHA-1 `c3b05f3d23a25baedce59a81445af3a5b2897309`.
- Environment: Windows build host; Rust 1.96.1; WiX 5.0.2; locked installer dependencies.
- Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1`.
- Actual: release Rust build, MSI and Burn bundle succeeded.
- MSI: 269,389,824 bytes; SHA-256 `5E8CDA73900D768812CB824E3EE31D369DE6C3E13D1DB34D423A7779C7B48445`.
- Setup: 557,246,731 bytes; SHA-256 `878B231C97F67F1140AE953C1D867E3C9EA94B4C4E4614DBF85A683981BA4132`.
- Secret scan: build output contains no configured runtime secret.
- Verdict: `PASS` as an unsigned test candidate; signing and current lifecycle evidence remain open.

### E-CI-01

- Gate: G2 workflow evidence.
- Repository: `mayas-alas/windows-rdp-tailscale`.
- Active branch/revision: `validation/gnx-dockur-lifecycle` at `4ed9075ca14e43c52037fe9e67b96c1775ed9c74`; the former branch name was retired after its commit history was preserved.
- Change: Ubuntu-hosted interactive compatibility lane with official Dockur image pinned by digest, read-only contents permission, frozen installer hash check, tailnet-only HTTPS noVNC, bounded session, user-invoked install/configure/evidence helpers, and redacted one-day artifact.
- Verification: actionlint, embedded helper self-tests, forbidden-marker scan, `git diff --check`, explicit two-file 0.1.3 asset-contract update, and successful branch push.
- Limitation: logical slots are independent ephemeral compatibility sessions and cannot demonstrate Corosync, quorum or G5 network acceptance.
- Verdict: `PASS` for the published workflow definition and frozen-asset contract; guest behavior is recorded separately.

### E-CI-02

- Gate: G2 hosted installer/noVNC execution; does not score runtime convergence or G5.
- Product source: `6822bce5e2a795f3f39a8dd379ea20315f880ac7`; workflow source: `4ed9075ca14e43c52037fe9e67b96c1775ed9c74`.
- Prerelease: `gnx-smoke-20260722095851`; setup asset SHA-256 `1D0F386C150E931EAF04634DB24999116281A9B19A33F5D0A1C1AD9A738BF53D`; MSI asset SHA-256 `191ED009FC8F50E2D8F77B4D6FAE9E399B93C8A237BB6BF98178C0DA275796E6`.
- Release URL: `https://github.com/mayas-alas/windows-rdp-tailscale/releases/tag/gnx-smoke-20260722095851`.
- Controller run URL: `https://github.com/mayas-alas/windows-rdp-tailscale/actions/runs/29936148126`.
- Member-1 run URL: `https://github.com/mayas-alas/windows-rdp-tailscale/actions/runs/29939246876`.
- Member-2 run URL: `https://github.com/mayas-alas/windows-rdp-tailscale/actions/runs/29942222291`.
- Environment: three independent Ubuntu 24.04 GitHub-hosted jobs using `dockurr/windows@sha256:8cc6f8bc4a60c078141fd3bcf7d2df69ae063a11d98be31a57429afe0dca66da`, KVM API 12, nested KVM enabled, TUN available, and tailnet-authenticated noVNC.
- End-user action: launch the frozen EXE through the guest helper, accept UAC, allow both installer reboots, recover the hosted guest after Windows automatic repair, finish the bundle, and run the allowlisted evidence collector as administrator.
- Expected: each logical slot installs the exact candidate, resumes after reboot, starts the Windows service, and exports schema-valid redacted evidence; Dockur is not expected to prove a shared cluster.
- Actual: all three workflow runs concluded `success`. Each guest installed the exact setup hash, resumed the bundle and reported `Installation Successfully Completed`. The installed service was `Running`, `Auto`, under `NT SERVICE\Quetzalcoatl`; `gnx.exe` SHA-256 was `675279BD0163F48ACCB4EE9566457200BC494A980A8216C863061A733E968CAB` in every slot.
- Hosted limitation: after WSL and Virtual Machine Platform were enabled, starting the Windows hypervisor sent each Dockur guest to automatic repair. `bcdedit /set {default} hypervisorlaunchtype off` restored Windows and allowed the bundle and evidence collector to finish, but necessarily disabled the nested hypervisor required by WSL2/Podman. Every exported `gnx status --json` therefore reported `overall=failed`, `stage=FAILED`, `role=null`, `service=ready`, `wsl=ready`, `podman_machine=failed`, `kvm=pending`, and all cluster fields false/pending.
- Retained artifacts: `target/evidence/run-29936148126-controller`, `target/evidence/run-29939246876-member-1`, and `target/evidence/run-29942222291-member-2`; each GitHub artifact was uploaded with one-day retention and a slot/run-specific name.
- Secret scan: no secret value was read or recorded; only required-secret presence was checked.
- Verdict: `PASS` for frozen-hash download, tailnet-only noVNC, real reboot/resume, MSI payload installation, service registration and redacted evidence export in all three logical slots. `FAIL` for hosted WSL2/Podman runtime convergence. Controller role, member roles, `PVE_JOIN=ready`, quorum and G5 remain explicitly unclaimed.

### E-PKG-03

- Gate: packaging prerequisite for G2/G5/G6; does not prove live installation or G5.
- Source revision: `a0b7a1a016c3545eda0ddcb76cb39477d7bbb82e`.
- Environment: Windows build host; Rust toolchain from `rust-toolchain.toml`; WiX 5.0.2.
- Contract: package, bundle, four Rust crates and helper CacheIds use 0.1.3; MSI identity is ProductCode `{2A1C371C-EDE5-48DE-A297-1EE70F18CD1C}`, PackageCode `{4931BD41-7686-4846-96A6-DFB5F1BB0AD8}` and preserved UpgradeCode `{47D5BD44-D061-407B-913B-47D17EC3BEA9}`; Burn registration ID is `{6FC46C58-8F5B-44E8-90D4-9E5E90A3EC33}`.
- Verification: `cargo fmt --all -- --check`, `cargo clippy -p gnx-service -- -D warnings`, `cargo test --workspace` (40 passed), PowerShell parse, two complete successful `installer/build.ps1` runs, direct byte comparison, positive reboot contract plus 10/10 negative mutations, static CRT import guard, shell syntax/static check and manifest digest for `gnx-pve-cluster-create`, MSI property query, Burn registration query and extracted-bundle payload comparison.
- MSI: 269,582,336 bytes; SHA-256 `191ED009FC8F50E2D8F77B4D6FAE9E399B93C8A237BB6BF98178C0DA275796E6`; ProductVersion `0.1.3`; ProductCode `{2A1C371C-EDE5-48DE-A297-1EE70F18CD1C}`; PackageCode `{4931BD41-7686-4846-96A6-DFB5F1BB0AD8}`; UpgradeCode `{47D5BD44-D061-407B-913B-47D17EC3BEA9}`.
- Setup: 557,536,137 bytes; SHA-256 `1D0F386C150E931EAF04634DB24999116281A9B19A33F5D0A1C1AD9A738BF53D`; Burn registration ID `{6FC46C58-8F5B-44E8-90D4-9E5E90A3EC33}`.
- Embedded payloads: both preflight entries equal release helper SHA-256 `F327D6F5F1B10689D221A17DF618D7EA9AFBA474A2A37EB837F989AA54564AC1`; embedded MSI equals the frozen MSI; WSL and Podman equal their dependency-lock hashes.
- Rebuild audit: `PASS`; two builds from the same source produced byte-identical MSI and EXE files. The build fixes PackageCode and Burn registration identity, normalizes MSI/CFB/CAB timestamps, recalculates the attached-container SHA-512 and CAB checksums, then extracts the result before accepting it.
- Secret scan: build and verification output contained no configured runtime secret.
- Verdict: `PASS` for the unsigned local packaging and reproducibility gate. Dockur installation, controller/member runtime behavior, signing and physical G5 remain unclaimed.

### E-RT-01

- Gate: controller runtime regression; no additional points awarded.
- Installed candidate: Quetzalcoatl 0.1.3 on a physical Windows 11 controller.
- Environment: managed Podman Machine under `NT SERVICE\Quetzalcoatl`; PVE,
  Tailscale Serve and KVM already ready.
- End-user action: run `gnx status` and `gnx status --json` after controller
  convergence stopped.
- Expected: the pinned OpenTofu one-shot creates the selected controller
  infrastructure and advances the controller toward `READY`.
- Actual: `overall=failed`, `stage=FAILED`, `role=controller`,
  `cluster.joined=true`, `cluster.quorate=true`, and
  `components.opentofu=failed`. `gnx-opentofu.service` exited 1 and the wrapper
  returned 67. Reverse-order journal capture plus the 1,600-character runtime
  bound retained Podman `container died/remove` events and discarded the
  underlying provider diagnostic.
- Corrective source change: serialize `tofu apply` with `-parallelism=1`;
  identify `init`, `validate`, and `apply` failures; preserve chronological
  journal order; exclude verbose Podman lifecycle events; update locked payload
  hashes.
- Corrective candidate revision:
  `40811611e058a7991b0e00a90aa9fc9d9e385594`.
- Local verification: shell syntax for both changed payloads; PowerShell parse;
  focused payload-manifest test; `cargo fmt --all -- --check`;
  `cargo clippy -p gnx-service -- -D warnings`; `cargo test --workspace`
  (40 passed); two successful complete `installer/build.ps1` runs; direct size,
  SHA-256 and byte comparison.
- Corrected entrypoint SHA-256:
  `A09EAD5B5B8D960D10412DA4B8B716E8C12A0C94F3E5C9AD760CE9FE7460F023`.
- Corrected prepare SHA-256:
  `5F1C31BA826BBCBF689B0D862CD26E62E9CEDEC95ADFC7681969D836C80AF40C`.
- Corrected manifest SHA-256:
  `6450816D7B4AA39E2252F3DA2C7850BB467EC93A88B352D6DE437D074A67C2C9`.
- Corrected MSI: 269,582,336 bytes; SHA-256
  `82D030A7147F9CA3DE36E5D4AB8681909F67E6500F1A70DE01F7201FC1AD3834`.
- Corrected Setup: 557,536,137 bytes; SHA-256
  `9A447111E213EC7C1FCB93668A2E2A2F43A3511218E55BE03C2C0AF4F998A510`.
- Rebuild audit: `PASS`; both corrected builds were byte-identical while
  using version 0.1.4, new MSI/Burn identities and the preserved UpgradeCodes.
- Artifact path: local installed `gnx status --json` response retained in the
  task transcript; no raw journal or environment was persisted.
- Secret scan result: status contained no configured secret value; protected
  runtime directories and service-owned WSL state were not accessed.
- Verdict: `FAIL` for the installed controller convergence, `PASS` for the
  local code/package correction, and `INCONCLUSIVE` for live behavior until
  the rebuilt 0.1.4 controller reaches `READY`.

### E-PKG-04

- Gate: packaging prerequisite for the 0.1.3 to 0.1.4 upgrade; live upgrade
  behavior remains part of G6.
- Source revision: `40811611e058a7991b0e00a90aa9fc9d9e385594`.
- Environment: Windows 11 build host; Rust 1.96.1; .NET SDK 8.0.423; WiX
  5.0.2; Visual Studio Build Tools 2022.
- Command: workspace format, Clippy and tests; two complete
  `installer/build.ps1` runs; direct size/SHA-256 comparison; Windows Installer
  property query; Burn extraction and registration/related-bundle query.
- Expected: 0.1.4 uses identities distinct from 0.1.3, preserves MSI and Burn
  UpgradeCodes, embeds the generated MSI and is byte-reproducible.
- Actual MSI: 269,582,336 bytes; SHA-256
  `82D030A7147F9CA3DE36E5D4AB8681909F67E6500F1A70DE01F7201FC1AD3834`;
  ProductVersion `0.1.4`; ProductCode
  `{EF6AC7CC-92A4-40C3-B8DA-D62845176E09}`; PackageCode
  `{0ED76FE6-9961-4D1A-8740-BAE336AFF777}`; preserved UpgradeCode
  `{47D5BD44-D061-407B-913B-47D17EC3BEA9}`.
- Actual Setup: 557,536,137 bytes; SHA-256
  `9A447111E213EC7C1FCB93668A2E2A2F43A3511218E55BE03C2C0AF4F998A510`;
  Burn ID `{A3A9B194-9B26-4C85-A240-AAA053B8D433}`; version `0.1.4`;
  preserved ProviderKey/related Upgrade bundle
  `{10B764B2-36AE-4911-A8C8-2F1A2A963769}`.
- Installed predecessor audit: Windows registers MSI ProductCode
  `{2A1C371C-EDE5-48DE-A297-1EE70F18CD1C}` and Burn ID
  `{6FC46C58-8F5B-44E8-90D4-9E5E90A3EC33}` at version `0.1.3`.
- Rebuild audit: `PASS`; both final MSI files and both final Setup files were
  byte-identical. The build also extracted the bundle and confirmed the
  embedded MSI and preserved upgrade relation.
- Artifact paths: `target/installer/Quetzalcoatl.msi` and
  `target/installer/QuetzalcoatlSetup.exe`.
- Secret scan result: build, MSI/Burn property queries and status evidence
  contained no configured secret value.
- Verdict: `PASS` for local unsigned packaging, identities and reproducibility;
  `INCONCLUSIVE` for the live in-place upgrade until 0.1.4 is run over the
  installed 0.1.3 controller.
