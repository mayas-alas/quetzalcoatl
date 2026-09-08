# PoC acceptance — 0.3.1

This document defines the minimum evidence required to call the current PoC functional. Documentation or compilation alone is not acceptance.

## Required result

From an authorized remote client:

1. `compute.gnx` resolves through the normal system resolver.
2. `https://compute.gnx` presents trusted TLS after the GNX public root is intentionally trusted.
3. Compute requires its own authentication and reports real health.
4. State survives reapply and reboot.
5. An optional external route can fail without breaking Access, Control or Compute.

## Gates

| Gate | Pass condition |
| --- | --- |
| G0 — Preflight | system runtime, cgroup v2, required devices, service generator and container runtime are usable; failures are actionable |
| G1 — Compute | service starts from the fixed release, authenticated health succeeds and persistent storage is available |
| G2 — Access | enrollment is persistent; reapply keeps the same private identity and DNS is authoritative for `.gnx` only |
| G3 — Control | `compute.gnx` works over verified TLS; direct Compute port and local control interfaces are not remotely exposed |
| G4 — Reconcile | unchanged apply is idempotent; invalid candidate configuration does not replace the last valid state |
| G5 — Recovery | Linux reboot and the documented Windows/WSL recovery path restore the three capabilities without recreating identity or storage |
| G6 — Parity | Windows `gnx.exe` and Linux `gnx` return the same operation semantics, JSON schema and exit codes |

## Evidence

For each run record only:

- GNX commit/version;
- host and kernel/runtime versions;
- immutable release references actually used;
- private identity/IP and selected backplane;
- public CA fingerprints;
- exact commands and results for G0–G6.

Never store passwords, enrollment credentials, cookies, private keys or authorization headers in evidence.

## Command contract

```text
gnx plan      # observation only
gnx apply     # reconcile desired state
gnx status    # local health
gnx doctor    # preflight and actionable diagnostics
```

Stdout is JSON. `READY` exits `0`, `FAILED` exits `1`, and `ACTION_REQUIRED` exits `2`.
