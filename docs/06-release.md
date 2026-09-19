# GNX 0.3.1 release definition

Status: normative  
Audience: release, build, audit  
Last reviewed: 2026-09-19  
Replaces: `release.md`, `lab-install-bundle.md`, artifact sections of evidence docs

A release is the authenticated, immutable implementation of a product contract. A build output or hash file alone is not a release.

## Candidate contents

A complete candidate contains or authenticates:

```text
release/
  manifest                     # schema, version, sizes, digests, platform matrix
  manifest.sig                 # detached Ed25519 signature over manifest bytes
  trust root                    # public key compiled into the setup boundary
  manifest authentication      # signature/attestation rooted in trusted key
  linux/
    gnx                        # application core
    runtime bundle             # Access/Control/Compute assets
  windows/
    gnx.exe                    # client bridge
    gnx-service.exe            # service/broker
    gnx-setup.exe              # bounded setup
    GNX Linux bundle           # same pipeline; inner digest recorded
    Wide Linux rootfs reference       # immutable identity and digest
  licenses and attribution
  schema and contract fixtures
```

The installer authenticates the manifest first, verifies every artifact before execution and records the signing identity in sanitized evidence.

## Intent versus release

| Operator `gnx.toml` | Internal release definition |
| --- | --- |
| schema, instance and node identity | release schema and GNX version |
| network intent and optional assertions | selected private transport implementation |
| explicit optional hostname/upstream routes | images, digests, licenses and source identities |
| no provider family, internal path or secret | systemd/Podman/CoreDNS/Caddy/runtime defaults |
| portable between compatible releases | platform artifacts, Wide Linux rootfs and required digests |

Different digest or runtime default means a different candidate.

## Build invariants

- Clean checkout plus declared inputs produces all first-party binaries.
- Linux and Windows contract tests use the same JSON fixtures.
- Windows package carries the exact Linux artifact produced by the candidate pipeline.
- No install-time download of unpinned replacements.
- Third-party runtime artifacts have immutable identity, digest, license and source recorded.
- Build-time Wide Linux/containers are compiler environments, never installed runtime state.
- Secrets and local acceptance credentials are not build inputs.
- The private release key is supplied only to the explicit sealing step through a protected environment reference; it is never committed, logged or passed in argv.

## Installation/update invariants

- Verify manifest authenticity and all staged digests before executing or importing.
- Install candidate beside/staged separately from last valid until health passes.
- `/usr/local/bin/gnx` is root-owned, not group/world writable and matches authenticated digest.
- Remove bootstrap copies after successful installation.
- Preserve private identity, GNX root, credentials and Compute data during update.
- Failed install/update returns `FAILED` or `ACTION_REQUIRED` and leaves last valid selected.

## Promotion checklist

A version may be labeled `0.3.1` PoC candidate only when:

- [ ] `07-implementation.md` M0-M7 exit conditions are satisfied or explicitly scoped for the candidate.
- [ ] Repository builds/tests from a clean checkout with locked inputs.
- [ ] Supported platform matrix is exact and `doctor` enforces it.
- [ ] Authenticated manifest and detached signature cover every installed/imported artifact.
- [ ] Linux and Windows output/exit fixtures are identical where required.
- [ ] Clean-host Linux installation completes without undocumented mutation.
- [ ] Clean-host Windows installation creates documented identity, service, ACL/DACL and isolated Wide Linux runtime.
- [ ] G0-G6 pass using `05-acceptance.md`.
- [ ] Evidence index contains no secrets and links every assertion to observation.
- [ ] Known limitations match non-goals and hide no acceptance exception.
- [ ] License, attribution and source obligations are present.

The release decision is binary: accepted or not accepted. A failed required gate requires a new candidate or explicit product decision; wording cannot be changed after the run to convert failure into success.

## LAB_ONLY bundles

Lab bundles may be used for Dockur or disposable-host validation. They must be labeled `LAB_ONLY`, include exactly the declared artifacts, record SHA256 values and never be promoted as release authenticity by themselves.

A strict six-artifact lab bundle pattern may contain:

```text
gnx.exe
gnx-service.exe
gnx-setup.exe
gnx-linux-bundle.tar
runtime-assets.tar
manifest.json
```

Rootfs references remain separate and must carry trusted digest. Lab evidence may prove staging/visibility/apply behavior but not producer authenticity unless tied to the signed release manifest.

## Historical reuse

Previous branches and legacy material may inform requirements, failure cases and test ideas. Code or behavior returns only when it:

1. traces to a current `BR-*` requirement;
2. fits current architecture and dependency rules;
3. uses the four-operation contract and JSON semantics;
4. has executable positive and refusal gates;
5. is freshly pinned, licensed and authenticated.

Historical success evidence does not transfer to a new candidate.
