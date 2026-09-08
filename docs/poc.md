# PoC acceptance — GNX 0.3.1

This document defines the minimum executable evidence required to call the clean
0.3.1 PoC functional. Documentation, compilation, generated configuration and a
successful process launch are not acceptance.

## Required result

From an authorized remote client:

1. `compute.gnx` resolves through the normal system resolver.
2. `https://compute.gnx` presents hostname-valid TLS after the GNX public root is
   deliberately verified and trusted.
3. Compute requires its own authentication and reports real service health.
4. Identity, last-valid configuration and Compute storage survive unchanged
   apply, service restart and host reboot.
5. An optional external route can fail without breaking Access, Control or
   Compute.

The result must be demonstrated on the exact candidate and platform versions
recorded in evidence. A local probe cannot substitute for the remote-client path.

## Test roles and baseline

The acceptance run uses distinct roles, even when a lab temporarily places more
than one role on the same physical machine:

| Role | Responsibility | Must not be used to bypass |
| --- | --- | --- |
| Build environment | produces and signs/attests immutable candidate artifacts | clean-host installation or artifact verification |
| GNX Linux runtime | owns application core, capability state and live services | Windows isolation and remote-client checks |
| Windows host | owns the service boundary and isolated `GNX` WSL distribution | Linux business logic or direct operator access to WSL state |
| Authorized remote client | exercises system DNS, private transport, TLS and Compute authentication | loopback-only or container-internal checks |
| Optional external application | proves route isolation outside GNX lifecycle | required Compute health |

Before a run, record the candidate commit and version, artifact manifest identity,
host versions, client versions and network prerequisites. Do not modify the
candidate after the run begins.

## Result contract

```text
gnx doctor    # host and release prerequisites
gnx plan      # desired versus observed state; no mutation
gnx apply     # transactional reconciliation
gnx status    # observed capability health
```

Every command writes one versioned JSON result to stdout. Human explanation goes
to stderr. The shared envelope must include at least:

```json
{
  "schema": 1,
  "operation": "status",
  "state": "READY",
  "code": "OK",
  "revision": "<last-valid-revision>",
  "capabilities": [],
  "next_action": null
}
```

The final schema is frozen in M0 and may add fields, but Linux and Windows must
use the same fixtures. `READY` exits `0`, `FAILED` exits `1`, and
`ACTION_REQUIRED` exits `2`. A parseable JSON failure is still a failure; an
unparseable or multiple-document stdout stream fails the contract.

## Gate summary

| Gate | Pass condition |
| --- | --- |
| G0 — Preflight | The supported host, authenticated release and runtime prerequisites are usable; every failure identifies a stable code and next action. |
| G1 — Compute | The fixed service starts, authenticated health succeeds and persistent storage is available. |
| G2 — Access | Enrollment is persistent; private reachability works; CoreDNS is authoritative for `.gnx` only over UDP and TCP. |
| G3 — Control | `compute.gnx` works over fully verified TLS; direct Compute ports and local Control interfaces remain unreachable. |
| G4 — Reconcile | Plan is read-only; unchanged apply is idempotent; invalid or unhealthy candidates do not replace last valid. |
| G5 — Recovery | Linux reboot and the documented Windows/WSL recovery path restore all required capabilities without recreating identity or storage. |
| G6 — Parity | Windows `gnx.exe` and Linux `gnx` implement the same operations, JSON schema, diagnostic semantics and exit codes. |

## G0 — Preflight and release integrity

Run `gnx doctor` before any mutation on each supported host path.

Required positive evidence:

- supported OS/kernel/WSL versions and architecture;
- usable cgroup v2, service generator, container runtime and required devices;
- sufficient declared memory, storage and required network bindings;
- readable, schema-valid intent and authenticated, schema-valid release;
- exact artifacts and digests selected for the platform;
- on Windows, dedicated-account prerequisites, SCM, named-pipe and WSL import
  prerequisites.

Required negative checks:

- corrupt one staged artifact and prove rejection before execution;
- provide an unauthenticated or incorrectly signed manifest and prove rejection;
- remove or deny one prerequisite and verify a stable failure code plus a
  corrective next action;
- provide an unsupported schema version and prove it is not guessed or silently
  downgraded.

G0 passes only when a clean candidate can proceed and the negative cases leave no
partial installation promoted as valid.

## G1 — Compute

Required positive evidence:

- the running artifact matches the release identity;
- lifecycle reports active after the configured start timeout;
- the health probe authenticates and confirms the expected service identity;
- persistent storage is available and a non-sensitive witness can be written and
  read;
- the private upstream contract exported to Control contains no credential or
  private key.

Required negative checks:

- an unauthenticated public page or open TCP port does not produce `READY`;
- invalid credentials, unavailable storage and an unexpected service identity
  each fail with different actionable codes;
- the upstream is not reachable directly from the remote-client network.

Record a non-secret service identity and storage witness digest before G5.

## G2 — Access and authoritative DNS

Required positive evidence:

- an authorized remote client reaches the private entry address;
- the node reports one persistent private identity;
- the normal client resolver returns the declared A record for `compute.gnx`;
- direct UDP and TCP queries receive authoritative `.gnx` answers;
- a declared optional route receives only its declared record.

Required negative checks:

- an unauthorized client cannot reach GNX service entrypoints;
- a name outside `.gnx` returns `REFUSED` from GNX rather than a recursive answer;
- an undeclared name within `.gnx` is not synthesized or forwarded;
- Access DNS does not bind to an unintended public/LAN interface;
- enrollment material is absent from argv, environment, intent, disk outside
  protected runtime state, logs, stdout and evidence.

Reapply Access and record that the private identity has not changed. The client
resolver test is required in addition to direct DNS queries because split-DNS
delivery is part of the product outcome.

## G3 — Control and end-to-end HTTPS

Required positive evidence:

- the operator exports only the public GNX root, verifies its fingerprint and
  deliberately installs trust on the test client;
- `https://compute.gnx` validates the requested hostname, validity period and GNX
  trust chain without an insecure override;
- Control connects to Compute using the declared upstream protocol and trust;
- Compute login and authenticated API/health work through Control;
- the required route remains healthy when the optional external application is
  stopped.

Required negative checks:

- before trust installation, the client rejects the GNX certificate;
- an unknown SNI name and an undeclared HTTP Host do not route to Compute;
- a wrong/expired upstream certificate or name fails closed;
- direct Compute ports, the container backplane and Control administration
  interfaces are unreachable from the remote client;
- stopping the optional application degrades only that route and produces an
  observable, non-success result.

TCP connectivity or `curl -k` is not G3 evidence. Capture the public certificate
fingerprints and sanitized HTTP/TLS results.

## G4 — Plan, apply and last-valid state

Run all cases from a known valid revision:

1. run `plan` twice and prove no managed file, unit, identity, credential,
   certificate, container or state revision changed;
2. run an unchanged `apply` twice and prove no unjustified restart or identity,
   credential, certificate, DNS serial or artifact churn;
3. stage syntactically invalid intent and prove rejection before runtime change;
4. stage semantically invalid intent, such as a duplicate hostname or a secret in
   a route URL, and prove rejection;
5. stage a candidate that renders correctly but fails its live health probe and
   prove that the previous route and last-valid revision still work;
6. interrupt apply at each recorded publication phase and prove a safe retry or
   deterministic restoration;
7. start a second apply and prove the per-node lock prevents concurrent
   reconciliation.

The gate passes when every failure preserves persistent capability state and the
last valid service remains or is restored to the documented extent.

## G5 — Recovery

Capture the Access identity, last-valid revision, GNX public root fingerprint,
Compute service identity and storage witness before recovery tests.

Linux path:

1. restart each managed service and verify `status`;
2. reboot the Linux runtime host;
3. wait only for documented bounded recovery;
4. rerun `doctor`, `status` and the remote G2/G3 path.

Windows path:

1. stop/start `GNXRuntime` and verify bounded SCM recovery;
2. execute the documented `wsl --shutdown` recovery path through the service;
3. reboot Windows without signing into the dedicated runtime account;
4. verify service identity, protected ACL, pipe DACL and `GNX` distro ownership;
5. verify automount and Windows interop remain disabled;
6. verify bootstrap artifacts are absent and `/usr/local/bin/gnx` still matches
   the authenticated release digest;
7. rerun G2/G3 from the remote client.

G5 passes only if every captured non-secret identity and witness remains the same
and no manual reimport, reenrollment or data recreation is needed.

## G6 — Linux/Windows parity and broker refusal

Run golden requests and failure fixtures through both binaries and compare the
normalized JSON and exit codes. Platform-specific diagnostic detail may differ
only in explicitly platform-scoped fields; operation meaning, state and stable
codes do not.

The Windows broker must also prove rejection of:

- an unknown protocol version or opcode;
- remote named-pipe clients;
- a caller outside the pipe DACL;
- truncated, oversized or trailing frames;
- arbitrary argv, shell text, Windows paths and WSL distribution names;
- a secret in the normal intent frame;
- response overflow or invalid Linux JSON.

A canary secret must be absent from process listings, environment, Windows and
Linux files outside protected intended state, logs, crash output, evidence and
returned JSON. The Linux core remains the sole owner of use-case orchestration.

## Evidence bundle

Each acceptance run records only:

- run identifier, UTC timestamps, GNX commit and version;
- host, kernel, WSL, runtime and client versions;
- authenticated manifest identity and immutable references actually used;
- hashes of candidate artifacts and installed `/usr/local/bin/gnx`;
- private identity/IP and selected release backplane, without enrollment data;
- public CA and server certificate fingerprints;
- exact commands, normalized JSON results and exit codes for G0-G6;
- before/after identity, revision and persistence witnesses;
- service account rights, service identity and protected ACL/DACL summaries;
- a pass/fail index linking every assertion to its raw sanitized observation.

Never store passwords, enrollment credentials, cookies, private keys,
authorization headers, session databases, service-account credentials or full
environment/process dumps in evidence. Evidence tooling must redact before
writing, not after publishing.

## Acceptance report

The final report contains:

| Field | Required value |
| --- | --- |
| Candidate | commit, version and manifest identity |
| Platforms | exact build host, Linux runtime, Windows host and remote client |
| Gates | G0-G6 with pass/fail and evidence link |
| Deviations | every manual action beyond documented `ACTION_REQUIRED` flows |
| Known limitations | explicit non-goals and untested conditions |
| Decision | accepted or not accepted; never “mostly passed” |

Any failed required assertion means the PoC is not accepted. It may still be a
useful diagnostic run, but the report must preserve that distinction.
