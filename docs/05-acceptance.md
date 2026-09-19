# GNX 0.3.1 acceptance and evidence

Status: normative acceptance protocol  
Audience: QA, audit, release  
Last reviewed: 2026-09-19  
Replaces: `poc.md`, `artifact-acceptance.md`, `dockur-*`, `persistent-windows-lab.md`, `final-integration-gate.md`

Acceptance is executable evidence, not documentation. A candidate is accepted only when the required gates pass on the declared platform matrix with exact artifacts and sanitized evidence.

## Required end-to-end result

From an authorized remote client:

1. `compute.gnx` resolves through the normal system resolver.
2. `https://compute.gnx` presents hostname-valid TLS after deliberate GNX public-root trust.
3. Compute requires its own authentication and reports real service health.
4. Identity, last-valid configuration and Compute storage survive unchanged apply, service restart and reboot.
5. Optional external route failure does not break Access, Control or Compute.

Local loopback, container-internal checks or noVNC availability cannot substitute for the remote-client path.

## Test roles

| Role | Responsibility | Must not bypass |
| --- | --- | --- |
| Build environment | produces authenticated candidate artifacts | clean-host install or artifact verification |
| GNX Linux runtime | owns application core and live services | Windows isolation |
| Windows host | owns service boundary and isolated WSL distro | Linux business logic |
| Authorized remote client | exercises DNS, private transport, TLS and Compute auth | loopback/container-only checks |
| Optional external app | proves route isolation | required Compute health |

## JSON contract gate

Every public command writes one JSON document using the shared envelope:

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

Linux and Windows use the same fixtures and exit semantics. Unparseable stdout, multiple JSON documents or success text outside JSON fails the gate.

## Gate summary

| Gate | Pass condition |
| --- | --- |
| G0 Preflight | supported host, authenticated release and prerequisites are usable; negative cases produce stable codes |
| G1 Compute | service starts, authenticated health passes and persistent storage is available |
| G2 Access | persistent enrollment, private reachability and authoritative `.gnx` DNS work from authorized client |
| G3 Control | `compute.gnx` works over verified TLS; private/local ports remain unreachable |
| G4 Reconcile | `plan` is read-only; unchanged `apply` is idempotent; invalid/unhealthy candidates do not replace last valid |
| G5 Recovery | Linux reboot and Windows/WSL recovery restore required capabilities without recreating identity/storage |
| G6 Parity | Windows `gnx.exe` and Linux `gnx` implement same operations, schema, diagnostics and exit codes |

## Required negative checks

- corrupt staged artifact: rejected before execution;
- unauthenticated/incorrectly signed manifest: rejected;
- unsupported schema: not guessed or silently downgraded;
- unauthenticated Compute page/open TCP port: not `READY`;
- unauthorized client: cannot reach service entrypoints;
- outside `.gnx`: refused, not recursively resolved;
- undeclared `.gnx` name: not synthesized or forwarded;
- direct Compute/local Control ports: unreachable from remote client;
- secret material: absent from argv, environment, intent, logs, stdout and evidence;
- broker opcode/path passthrough: rejected.

## Evidence index requirements

### Windows lifecycle gate

Use the [actionable lifecycle and uninstall checklist](../packaging/windows/uninstall-checklist.md) for ownership, the twelve lifecycle classifications, failure injections and retry decisions. Run `powershell -NoProfile -File packaging/windows/setup-ui.tests.ps1` from the candidate checkout and record its exit code. Missing packaging scripts or failed assertions fail the static gate; a passing static gate does not prove installation, removal or runtime health.

On the declared disposable Windows matrix, retain sanitized pre/post observations for provision, interruption in each mutation phase, ownership conflicts, explicit recovery, removal, repeated removal, reboot and reinstall. Query SCM, local account SID/references, WSL registration in its owner's context, protected roots, shortcuts, tasks and applicable Path/Run scopes independently of the command under test. A query error is not absence; any residue or contradiction between exit/result/counters and observations fails the gate. Preserve foreign sentinels unchanged and record retained evidence outside the removed roots.

The setup `--json-progress` transport is an explicit NDJSON stream exception to the single-document command gate: require exactly one `started` followed by one `completed`, matching requested operation and terminal state/exit, with no extra output. Negative fixtures must reject duplicates, reversed phases, operation mismatch, malformed JSON and false `apply_available`. Existing source checks verify guards and mappings, but do not substitute for executing those fixtures; missing rejection is a reported transport gap.

Record `PROVISIONED` as `ACTION_REQUIRED` / `SETUP_PROVISIONED` / exit 2, never as runtime READY. Record REMOVED only after independent absence checks and reboot observations pass. Lifecycle classifications are separate from public envelope states and must be labelled as observed, inferred or unavailable.

Each acceptance run records candidate commit/version, manifest/signing identity, artifact digests, platform/client versions, exact commands or scripted steps, positive and negative gate observations, sanitized logs/results and cleanup or retained-state decision.

A failed gate is reported as failed or blocked. It is never rewritten as success.

## Current evidence status

As of this consolidation:

- Latest commit observed before docs cleanup: `b2f0474 docs: record full Windows apply acceptance`.
- Local `cargo test` under default MSVC target was **BLOCKED** because `link.exe` resolved incorrectly/missing MSVC build tools. This is an environment/toolchain blocker, not a passed code gate.
- Previous final integration evidence passed formatting/static/script checks but kept host acceptance as pending.
- Host cleanup removed old active `C:\Program Files\GNX` and `C:\ProgramData\GNX` by moving them to a timestamped backup; no GNX process/service remained afterward.

## Dockur Windows evidence summary

Dockur was used only as a Windows lab/acceptance environment, not as product runtime.

Retained facts:

- Pinned image digest used in prior runs: `docker.io/dockurr/windows@sha256:0cff9eb0e7aee9953e55bc682852ca4fdca233145a58ae1ec94f0b0c01a2ed30`.
- Safe pattern: disposable or retained Podman volume, `/dev/kvm`, `/dev/net/tun` only when required, web bound to loopback, temporary share, no secrets or legacy host mounts.
- Initial bounded attempts were blocked while Windows ISO was still downloading and QEMU was absent; no guest readiness was claimed.
- Follow-up boot reached Windows desktop and proved `/shared` visibility of GNX EXEs; GNX was not installed/exercised in that run.
- A later persistent lab reached real setup/apply work and produced expected finite boundary `ACTION_REQUIRED SETUP_REBOOT_REQUIRED`; after reboot, WSL-required Windows features were disabled, then nested virtualization/boot-manager behavior blocked further `READY` claim.

Dockur evidence may support G0/G6 lab diagnosis only when the exact candidate, artifacts and guest observations are recorded. It cannot replace clean physical/declared Windows host acceptance unless the release explicitly declares that matrix.

## Dockur safe procedure

Use this pattern for future lab runs:

```text
host staging dir -> Dockur /shared -> Windows Shared/Z:
host OEM dir     -> Dockur /oem    -> Windows C:\OEM
podman logs      -> host evidence
guest JSON/logs  -> written back to /shared
```

Rules:

- do not mount `C:\Program Files\GNX`, `C:\ProgramData\GNX`, `legacy`, `/var/lib/gnx` or secrets;
- use unique container/volume/staging names;
- bind web UI to loopback;
- record hashes of staged artifacts;
- have guest scripts write small sanitized JSON (`phase`, `state`, `code`, `exit`) to the share;
- clean disposable resources exactly, or explicitly document retained lab state.
