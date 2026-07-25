# Member membership — 0.1.15

## Discovery rule

- No GNX peers: become the controller.
- Existing peers with exactly one `gnx-controller-*`: become a member of that controller.
- Existing peers with no controller or more than one controller: fail closed.

The number of existing members is not used as a rejection rule.

## Join sequence

```text
MEMBER_PREPARING
→ MEMBER_AUTHORIZING
→ MEMBER_JOINING
→ MEMBER_VERIFYING
→ MEMBER_CONFIRMING
→ READY
```

`MEMBER_JOINING` preserves the existing allowlisted and idempotent `pvecm add` operation. A reboot or rerun inspects the real cluster state before repeating destructive work.

`MEMBER_VERIFYING` validates the local PVE identity and services. `MEMBER_CONFIRMING` invokes one typed Fedora-agent operation that checks cluster name, quorum, `pvecm nodes`, local topology and PVE cluster resources for both controller and member.

## Lean authorization boundary

`authorize-member` is a reconciler decision based on protected configuration and persisted controller/member identity. It is not a new service endpoint, token protocol or public CLI command.

## Deliberately absent

- QDevice and HA automation;
- arbitrary cluster commands;
- node removal or forced repair;
- a controller-side GNX listener;
- protocol or state-schema migration.
