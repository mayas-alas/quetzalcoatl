# Changelog

## 0.1.17 buildfix-03 — remote-execution policy consolidation

- Formalized the normative argv/stdin/file boundary and added a mandatory review checklist.
- Corrected stale topology and lifecycle documentation to match the implemented controller-presence and PVE-before-Serve behavior.
- Extended source validation to reject shell-control syntax in explicit direct remote argv arrays.
- Changed no runtime executable, installer identity, protocol, persisted state, payload or network surface.

## 0.1.17 buildfix-02 — Serve stdin repair

- Removed nested `sh -c` and `< /config/serve.json` from Serve application.
- Serialized the fixed PVE Serve route in Rust and sent it through bounded stdin to `podman exec -i ... tailscale serve set-raw`.
- Strengthened multiline `sh`/`bash -c` detection and added Serve contract coverage.

## 0.1.17 — installed MVP stabilization

- Replaced fixed 6 CPU, 8192 MiB and 100 GiB Podman Machine values with a bounded host profile calculated during preflight.
- Persisted one atomic `host-profile.json` for both service `.wslconfig` and Podman Machine creation.
- Classified hosts as `install-only`, `lab`, `runtime` or `cluster-member` and added deterministic coverage for the observed 5864 MiB host.
- Simplified first-install role discovery to online controller presence: zero controllers promotes the node; one or more controllers creates a member.
- Removed member count, candidates and unrelated tagged peers from the role decision.
- Accepted the GNX tag by membership instead of exact tag-list equality.
- Committed role state only after observing the final Tailscale hostname.
- Deferred Serve activation until the fixed local PVE backend is ready.
- Preserved protocol schema 2, persisted-state schema 2, runtime payload 5, runtime generation and the four-package workspace.

## 0.1.15 — installer recovery and member confirmation

- Replaced direct Burn execution of the pinned WSL and Podman MSIs with closed host-preflight helpers.
- Added fixed-name, exact-size and SHA-256 validation plus stable staging under `ProgramData` before `msiexec`.
- Added persistent dependency MSI logs and a product-version-scoped, three-attempt install journal.
- Added `gnx -v` and `gnx --version` as local CLI actions that do not contact the service.
- Added persisted member prepare, authorize, verify and confirm stages around the existing idempotent join.
- Added one allowlisted runtime-agent membership-confirmation operation and bumped the exact payload contract to version 5.
- Removed the topology rejection based on member count while preserving the requirement for exactly one controller.
- Preserved protocol schema 2, persisted-state schema 2, runtime generation and the four-package workspace.

## 0.1.13 — CLI contract and source hygiene

- Removed inactive `runtime_gate.rs` and `remote/process.rs` legacy sources.
- Removed closed 0.1.11/0.1.12 delivery records from the active `.AGENTS` scope.
- Preserved the MVP CLI command set: `status`, `configure` and `restart`.
- Expanded human `gnx status` output to include controller, Tailscale, Proxmox and cluster quorum fields.
- Added CLI protocol schema guards for status and configuration responses.
- Added source and MSI contracts for `gnx.exe`, including PATH registration and extracted-binary hash verification.
- Added a dedicated CLI contract validator and 0.1.13 audit report.
- Updated 0.1.13 MSI/Burn identities while retaining the established upgrade families.

## 0.1.12 — MVP consolidation

- Extracted runtime orchestration into `reconciler.rs` without changing stage order.
- Separated runtime errors from data models.
- Replaced free-form Fedora-agent argv construction with a closed `RuntimeOperation` enum.
- Added remote stdin/output limits and a 15-minute process timeout.
- Removed managed-runtime `sh -c` execution, including PVE credential configuration.
- Bumped the exact payload manifest contract to version 4 and regenerated hashes.
- Split Rust build and payload-source verification into installer modules.
- Added local repository, runtime, remote-execution and release-contract validators.
- Updated 0.1.12 MSI/Burn identities while retaining upgrade families.

## 0.1.11 — scoped modularization

- Split the runtime monolith inside `gnx-service`.
- Consolidated one runtime payload and added the on-demand Fedora agent.
- Split installer build helpers without changing its entry point.

## 0.1.17

- Simplifies first-install role discovery to controller presence only.
- Existing members and transient candidates no longer block enrollment.
- Zero online controllers promotes the node to controller; one or more online controllers makes it a member, selecting deterministically by Tailscale node ID.
- Accepts the GNX tag by membership instead of requiring an exact single-tag list.
- Ignores stale, malformed and offline tagged peers outside the controller decision path.
- Commits persistent role state only after the final Tailscale hostname is observed.
- Defers Tailscale Serve activation until the PVE backend is ready.
- Separates local Tailscale readiness errors from enrollment errors.
