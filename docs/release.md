# Release 0.3.1 — clean PoC baseline

This branch is a documentation-first reset. It intentionally contains no
inherited product implementation. Its purpose is to freeze enough product,
architecture and acceptance detail for a new implementation to begin without
silently restoring the previous runtime.

## Candidate status

| Item | Status |
| --- | --- |
| Product requirements | defined |
| Architecture and target tree | defined |
| Windows isolation boundary | defined |
| Implementation sequence | defined |
| G0-G6 acceptance protocol | defined |
| Runtime implementation | absent |
| Reproducible release artifacts | absent |
| Executed acceptance evidence | absent |

The baseline is therefore **not a PoC release candidate**. It becomes a candidate
only after the implementation and promotion conditions below are met.

## Product scope

- Public capabilities are only **Access / Control / Compute**.
- The stable Compute entrypoint is `https://compute.gnx`.
- The public use cases are `plan`, `apply`, `status` and `doctor`.
- One Linux application core owns reconciliation; Windows is a typed WSL bridge.
- Node intent does not select implementations or release artifacts.
- Release-specific immutable references live in one internal release definition.
- CoreDNS is the fixed private DNS adapter for this PoC and is authoritative only
  for `.gnx`.
- The Windows bundle contains `gnx.exe`, `gnx-service.exe` and the GNX Linux
  binary produced by the same pipeline. An authenticated manifest pins them, the
  runtime assets and the WSL rootfs before installation.

## Release definition versus operator intent

The operator supplies desired node intent. The release supplies every choice
required to make that intent executable.

| Operator `gnx.toml` | Internal release definition |
| --- | --- |
| schema, instance and node identity | release schema and GNX version |
| network intent and optional assertion | selected private transport implementation |
| explicit optional hostname/upstream routes | exact images, digests and licenses |
| no provider family, internal path or secret | systemd/Podman/CoreDNS assets and defaults |
| portable between compatible 0.3.1 releases | platform artifacts, WSL rootfs and required digests |

The release definition is immutable for a candidate. A different digest or
runtime default creates a different candidate even when the operator intent is
unchanged.

## Required release contents

A complete candidate contains or authenticates all of the following:

```text
release/
├── manifest                         # versioned identities, sizes and digests
├── manifest authentication          # signature/attestation rooted in a trusted key
├── linux/
│   ├── gnx                          # application core
│   └── runtime bundle               # fixed capability assets
├── windows/
│   ├── gnx.exe                      # client bridge
│   ├── gnx-service.exe              # SCM service and broker
│   ├── GNX Linux bundle             # same pipeline, inner digest recorded
│   └── WSL rootfs reference         # immutable identity and digest
├── licenses and attribution
└── schema and contract fixtures
```

A digest file shipped next to unsigned artifacts proves integrity only against
that file; it does not prove producer authenticity. The installer authenticates
the manifest first, verifies every artifact before execution and records the
signing identity in sanitized evidence.

## Build invariants

- A clean checkout plus declared build inputs produces all first-party binaries.
- Linux and Windows contract tests use the same JSON fixtures.
- The Windows package embeds or carries the exact Linux artifact produced by the
  candidate pipeline; it does not download an unpinned replacement at install
  time.
- Every third-party runtime artifact has an immutable identity, digest, license
  and source recorded in the manifest/SBOM.
- Build-time WSL or containers are compiler environments, never copied as
  operator configuration or accepted as the installed runtime.
- Secrets, local state and acceptance credentials are not build inputs.

## Installation and update invariants

- Installation verifies manifest authenticity and all staged digests before
  executing a staged binary or importing a rootfs.
- A candidate is installed beside or staged separately from the last valid
  runtime until post-install health passes.
- `/usr/local/bin/gnx` is root-owned, not group/world writable and matches the
  inner digest authenticated by the release manifest.
- Bootstrap copies are removed after successful installation.
- Update preserves private identity, the GNX root, credentials and Compute data.
- A failed install or update returns `FAILED` and leaves the last valid runtime
  selected; it never reports a partially updated system as `READY`.

## Promotion checklist

A version may be labeled `0.3.1` PoC candidate only when:

- [ ] M0-M7 in `implementation-plan.md` satisfy their exit conditions.
- [ ] The repository builds and tests from a clean checkout with locked inputs.
- [ ] The supported platform matrix is exact and `gnx doctor` enforces it.
- [ ] The authenticated manifest covers every installed or imported artifact.
- [ ] Linux and Windows output/exit fixtures are identical where required.
- [ ] Clean-host Linux installation completes without undocumented mutation.
- [ ] Clean-host Windows installation creates the documented identity, service,
      ACL/DACL and isolated WSL runtime.
- [ ] G0-G6 pass on the candidate using the protocol in `poc.md`.
- [ ] The evidence index contains no secret and links every assertion to an
      observation.
- [ ] Known limitations match the documented non-goals and contain no hidden
      acceptance exception.
- [ ] License, attribution and source obligations are present.

The release decision is binary: accepted or not accepted. A failed required gate
cannot be waived by changing its wording after the run; it requires a new
candidate or an explicit product decision reviewed against business requirements.

## Deliberate reset and historical reuse

Previous source code, runtime units, packaging scripts, tests and evidence are not
carried into this branch. Earlier branches may be consulted to recover reasoning
and failure cases. Code or behavior returns only when all of these are true:

1. it traces to a current `BR-*` requirement;
2. it fits the current target tree and dependency rules;
3. it uses the four-operation contract and current JSON semantics;
4. it has an executable current gate, including its refusal path;
5. its release artifacts are freshly pinned and authenticated.

The detailed history review is in `documentation-audit.md`. Historical success
evidence does not transfer to a new candidate.

## Non-goals

No provider/plugin framework, workload catalog, VM/LXC provisioning, scheduler,
HA, automatic upgrades, tray application or commercial installer. The PoC does
not claim isolation from Linux root, Windows SYSTEM/local Administrators or a
compromised privileged runtime component.

## Verification status

No implementation exists on this baseline, so G0-G6 are not run and the PoC is
not accepted. The first implementation change must add executable evidence for
its own contracts rather than converting missing runtime checks into
documentation claims.
