# Quetzalcoatl 0.1.17 lean topology audit

## Decision contract

For a new installation, GNX reads `tailscale status --json` and validates the local node. It then considers only valid, online `gnx-controller-*` peers carrying `tag:quetzalcoatl-node`:

- no controller: promote the local node to controller;
- one or more controllers: create a member and select the controller deterministically by stable Tailscale node ID;
- existing members, candidates, stale peers and unrelated tagged peers do not affect this decision.

For upgrades with valid persisted state, GNX preserves the existing role and validates the persisted controller for members.

## Core changes

- Removed global tagged-peer strictness from initial discovery.
- Replaced exact tag-list comparison with tag membership.
- Removed member count and multiple-controller errors from role selection.
- Deferred state commit until Tailscale confirms the renamed local identity.
- Removed automatic `TS_SERVE_CONFIG` activation from the sidecar.
- Added explicit Serve application after PVE readiness.

## Deliberate MVP boundary

This remains a single-cluster-per-tailnet model. When several controllers already exist, GNX chooses the lowest stable node ID instead of blocking installation. Controller convergence and multi-cluster identities remain outside 0.1.x.

## Certification boundary

Static Python validators and clean-archive checks are included. Cargo, Clippy, Windows service execution and WiX builds must be run on the Windows build host.
