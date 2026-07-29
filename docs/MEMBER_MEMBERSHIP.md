# Member membership — 0.1.17

## Discovery rule

For a new installation, GNX validates local Tailscale readiness and searches only for valid online `gnx-controller-*` peers carrying `tag:quetzalcoatl-node`.

- No online controller: become the controller.
- One or more online controllers: become a member and select the controller deterministically by stable Tailscale node ID.
- Existing `gnx-member-*` peers do not affect the decision and do not impose a count limit.
- Candidates, expired peers, offline controllers, malformed peers and unrelated tagged peers are ignored by the initial role decision.

This is deliberately a lean single-cluster-per-tailnet rule. Multiple controllers indicate external drift, but 0.1.17 does not block a new member because of their count.

## Join sequence

```text
MEMBER_PREPARING
→ MEMBER_AUTHORIZING
→ MEMBER_JOINING
→ MEMBER_VERIFYING
→ MEMBER_CONFIRMING
→ READY
```

`MEMBER_JOINING` preserves the allowlisted and idempotent `pvecm add` operation. A reboot or rerun inspects real PVE state before repeating mutation.

`MEMBER_VERIFYING` validates local PVE identity and services. `MEMBER_CONFIRMING` invokes the typed Fedora-agent operation that checks cluster name, quorum, `pvecm nodes`, local topology and PVE cluster resources.

## Upgrade rule

A valid persisted member does not rediscover or replace its controller during an upgrade. GNX validates that the persisted controller remains compatible and available before continuing.

## Lean authorization boundary

`authorize-member` is a local reconciler validation based on protected configuration and persisted identities. It is not a controller-side approval API, token protocol or public CLI command.

## Deliberately absent

- controller failover or distributed election;
- multi-cluster identity within one tailnet;
- QDevice and HA automation;
- arbitrary cluster commands;
- node removal or forced repair;
- a controller-side GNX listener;
- protocol or state-schema migration.
