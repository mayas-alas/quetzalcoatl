# Release 0.3.1 — clean PoC baseline

This branch is a documentation-only reset. It intentionally contains no inherited
product implementation.

## Product scope

- Public capabilities are only **Access / Control / Compute**.
- The stable Compute entrypoint is `https://compute.gnx`.
- The public use cases are `plan`, `apply`, `status` and `doctor`.
- One Linux application core owns reconciliation; Windows is a typed WSL bridge.
- Node intent does not select implementations or release artifacts.
- Release-specific immutable references live in one internal release definition.
- CoreDNS is the fixed private DNS adapter for this PoC and is authoritative only
  for `.gnx`.

## Deliberate reset

Previous source code, runtime units, packaging scripts, tests, evidence and legacy
documentation are not carried into this branch. They may be consulted through Git
history, but new implementation must be justified by
`business-requirements.md`, respect `architecture.md`, and pass `poc.md`.

## Non-goals

No provider/plugin framework, workload catalog, VM/LXC provisioning, scheduler,
HA, automatic upgrades, tray application or commercial installer.

## Verification status

No implementation exists on this baseline, so G0-G6 are not run and the PoC is
not accepted. The first implementation change must add executable evidence rather
than converting missing runtime checks into documentation claims.
