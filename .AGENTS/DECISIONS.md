# Architecture and orchestration decisions

## D-001 - Agent boundary

Accepted 2026-07-21. All implementation, test, CI, and documentation workers are separate `codex exec` processes. Native subagents are prohibited except when Graphify itself requires them for semantic graph generation or update.

## D-002 - MVP topology

Accepted 2026-07-21. The release gate is exactly one controller plus two members. Member code is generic, but fourth-node acceptance does not count toward this cycle and must not delay closure.

## D-003 - Evidence hierarchy

Accepted 2026-07-21. Dockur on a Linux GitHub runner is the required CI compatibility lane. Windows-native GitHub runners are not a required release signal. Three real consumer Windows hosts on a low-latency site are the sole authority for Corosync and quorum acceptance.

## D-004 - Documentation authority

Accepted 2026-07-21; consolidated 2026-07-22. `docs/ARCHITECTURE.md` is the normative technical contract, `.AGENTS/SCOPE.md` fixes the product boundary, `docs/VALIDATION.md` is the acceptance runbook, `.AGENTS/TRACKER.md` is the only progress tracker and `.AGENTS/EVIDENCE.md` is the evidence ledger. Retired specifications, prompts and task fragments are historical Git objects, not active sources.

## D-005 - Score integrity

Accepted 2026-07-21. Points are awarded only for reproducible evidence. Missing physical three-host evidence caps the score at 70/100. Any secret exposure, Windows-published Proxmox port, non-direct Tailscale cluster path, or non-quorate cluster blocks release regardless of numeric score.

## D-006 - CLI reasoning allocation

Accepted 2026-07-21. Codex CLI agents use `medium` reasoning for runtime integration, CI, security, secrets, recovery, and Proxmox work. `low` is reserved for narrow mechanical edits with deterministic acceptance. Agent self-reports never replace architect review.

## D-007 - Reproducible release identity

Accepted 2026-07-22; expanded by explicit authorization on 2026-07-23.
Quetzalcoatl 0.1.4 has MSI ProductCode
`{EF6AC7CC-92A4-40C3-B8DA-D62845176E09}`, PackageCode
`{0ED76FE6-9961-4D1A-8740-BAE336AFF777}`, preserved UpgradeCode
`{47D5BD44-D061-407B-913B-47D17EC3BEA9}` and Burn registration ID
`{A3A9B194-9B26-4C85-A240-AAA053B8D433}`. Package, bundle, Rust crates and
CacheIds use the same version. The build rejects identity drift and verifies
the generated MSI and Burn identities. The prior 0.1.3 identities remain
historical and must not be reused for changed payloads.

WiX 5 normally generates fresh bind identities and timestamps. The scoped packaging extension fixes the Burn registration ID; the build fixes MSI summary identity/timestamps, normalizes compound-file and cabinet timestamps, updates the attached-container hash and recalculates cabinet checksums. A release candidate is accepted only after two builds are byte-identical and its exact SHA-256 values are recorded.
