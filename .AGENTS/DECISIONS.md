# Architecture decisions

## D-001 — Product completion is cluster READY

Accepted 2026-07-23. Convergence completes after controller cluster verification or member join.

## D-002 — Persist only the cluster contract

Protected state contains node identity, role, controller identity, tailnet and join checkpoint. Supplementary schema-one fields are not written into schema two.

## D-003 — Runtime payload is exact and minimal

Every installed runtime file must appear exactly once in both the Rust `PAYLOAD_FILES` contract and the locked manifest with an exact SHA-256.

## D-004 — Resume verifies rather than recreates

A controller with a persisted cluster checkpoint must verify the existing cluster before final `READY`. A member must resume against its pinned controller.
