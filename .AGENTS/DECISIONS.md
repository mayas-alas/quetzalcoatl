# Architecture and orchestration decisions

## D-001 - Agent boundary

Accepted 2026-07-21. All implementation, test, CI, and documentation workers are separate `codex exec` processes. Native subagents are prohibited except when Graphify itself requires them for semantic graph generation or update.

## D-002 - MVP topology

Accepted 2026-07-21. The release gate is exactly one controller plus two members. Member code is generic, but fourth-node acceptance does not count toward this cycle and must not delay closure.

## D-003 - Evidence hierarchy

Accepted 2026-07-21. Dockur on a Linux GitHub runner is the required CI compatibility lane. Windows-native GitHub runners are not a required release signal. Three real consumer Windows hosts on a low-latency site are the sole authority for Corosync and quorum acceptance.

## D-004 - Documentation authority

Accepted 2026-07-21. `docs/ARCHITECTURE.md` is the normative technical contract. `quetzalcoatl-cierre-poc-mvp.md` supplies the closure checklist until its consistent content is migrated. `.AGENTS/TRACKER.md` is the execution tracker. `PoC.md` and `docs/TRACKING.md` become legacy summaries after their still-valid evidence is migrated.

## D-005 - Score integrity

Accepted 2026-07-21. Points are awarded only for reproducible evidence. Missing physical three-host evidence caps the score at 70/100. Any secret exposure, Windows-published Proxmox port, non-direct Tailscale cluster path, or non-quorate cluster blocks release regardless of numeric score.

## D-006 - CLI reasoning allocation

Accepted 2026-07-21. Codex CLI agents use `medium` reasoning for runtime integration, CI, security, secrets, recovery, and Proxmox work. `low` is reserved for narrow mechanical edits with deterministic acceptance. Agent self-reports never replace architect review.
