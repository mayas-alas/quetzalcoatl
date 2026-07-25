# Agent execution contract — 0.1.15

This directory records three bounded workstreams and one integration gate. Each workstream has exclusive ownership so that installer recovery, CLI contracts and cluster membership cannot silently rewrite each other.

## Workstreams

1. `agents/installer-recovery.md` — stable dependency staging, reboot recovery and installer evidence.
2. `agents/cli-release-contract.md` — local version flags, public CLI compatibility and release identities.
3. `agents/cluster-membership.md` — member preflight, authorization decision, verification, confirmation and multiple-member discovery.

The integrator owns shared Cargo/WiX files, payload manifest regeneration, cross-workstream validation and the final source archive.

Closed 0.1.13 execution records remain under `.AGENTS/archive/0.1.13/` and are not active instructions.
