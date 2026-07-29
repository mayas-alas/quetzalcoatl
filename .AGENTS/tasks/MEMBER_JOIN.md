# Task: controlled member join

## Required sequence

`MEMBER_PREPARING → MEMBER_AUTHORIZING → MEMBER_JOINING → MEMBER_VERIFYING → MEMBER_CONFIRMING → READY`

The existing typed join operation remains responsible for the idempotent `pvecm add`. Confirmation must inspect cluster name, quorum, `pvecm nodes` and PVE cluster resources.

A new node enters this sequence whenever at least one valid online controller is visible. It selects deterministically by stable Tailscale node ID. Existing member count does not participate in discovery or rejection.

No new network API, schema migration or arbitrary controller command is allowed.
