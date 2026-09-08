# PoC acceptance — GNX 0.3.1

This document defines the minimum evidence required to call the clean 0.3.1 PoC
functional. Documentation, compilation and generated configuration are not
acceptance.

## Required result

From an authorized remote client:

1. `compute.gnx` resolves through the normal system resolver.
2. `https://compute.gnx` presents trusted TLS after the GNX public root is
   intentionally trusted.
3. Compute requires its own authentication and reports real health.
4. Identity, configuration and Compute storage survive reapply and reboot.
5. An optional external route can fail without breaking Access, Control or
   Compute.

## Gates

| Gate | Pass condition |
| --- | --- |
| G0 — Preflight | System runtime, cgroup v2, required devices, service generator and container runtime are usable; failures are actionable. |
| G1 — Compute | Service starts from the fixed release, authenticated health succeeds and persistent storage is available. |
| G2 — Access | Enrollment is persistent; reapply keeps the same private identity; CoreDNS answers authoritatively for `.gnx` only through UDP and TCP. |
| G3 — Control | `compute.gnx` works over verified TLS; direct Compute ports and local Control interfaces are not remotely exposed. |
| G4 — Reconcile | Unchanged apply is idempotent; invalid candidate intent does not replace the last valid state. |
| G5 — Recovery | Linux reboot and the documented Windows/WSL recovery path restore the three capabilities without recreating identity or storage. |
| G6 — Parity | Windows `gnx.exe` and Linux `gnx` return the same operation semantics, JSON schema and exit codes. |

G2 must also prove that a name outside `.gnx` is refused rather than recursively
resolved. G3 must prove hostname verification, not only TCP reachability.

On Windows, G0 also verifies the authenticated manifest and every staged digest.
G5 verifies the dedicated account, protected state ACL, `GNXRuntime` automatic
recovery, isolated WSL ownership and removal of bootstrap artifacts. G6 rejects
unknown broker opcodes, remote pipe clients, oversized frames and arbitrary argv;
it also proves that a secret is absent from process listings, environment, disk,
logs, evidence and returned JSON.

## Command contract

```text
gnx plan      # observation only
gnx apply     # reconcile desired state
gnx status    # local health
gnx doctor    # preflight and actionable diagnostics
```

Stdout is JSON. `READY` exits `0`, `FAILED` exits `1`, and `ACTION_REQUIRED`
exits `2`. The schema and exit code are part of G6.

## Evidence

For each run record only:

- GNX commit and version;
- host, kernel and runtime versions;
- immutable release references actually used;
- private identity/IP and selected backplane;
- public CA fingerprints;
- exact commands and results for G0-G6;
- service account rights, service identity and protected state ACL, without
  recording its generated password;
- hashes and signature identity from the authenticated release manifest;
- installed `/usr/local/bin/gnx` digest and the digest expected by the release.

Never store passwords, enrollment credentials, cookies, private keys or
authorization headers in evidence.
