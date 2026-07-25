# Task: controlled member join

## Required sequence

`MEMBER_PREPARING → MEMBER_AUTHORIZING → MEMBER_JOINING → MEMBER_VERIFYING → MEMBER_CONFIRMING → READY`

The existing typed join operation remains responsible for the idempotent `pvecm add`. Confirmation must inspect cluster name, quorum, `pvecm nodes` and PVE cluster resources. Additional members are permitted when exactly one compatible controller is visible.

No new network API, schema migration or arbitrary controller command is allowed.
