# Agent: cluster membership

## Ownership

- `crates/gnx-service/src/runtime/cluster/`
- member coordination sections of `runtime/topology.rs`
- the allowlisted membership-confirmation runtime operation
- `ci/validate_cluster_contract.py`
- `docs/MEMBER_MEMBERSHIP.md`

## Mission

Keep the existing idempotent `pvecm add` path and surround it with persisted prepare, authorize, verify and confirm phases. Allow multiple members while requiring exactly one controller. Confirm both nodes through cluster state without a new GNX endpoint.

## Prohibited

- arbitrary shell/argv execution;
- a controller HTTP/HTTPS listener;
- public member-administration CLI commands;
- schema migration, QDevice or HA logic;
- node removal or forced cluster repair.

## Result

Integrated. The runtime payload allowlist contains one typed `confirm-member` operation; the member is marked `READY` only after local verification and cluster-state confirmation.
