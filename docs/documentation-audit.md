# Documentation and history audit — GNX 0.3.1

- **Audit date:** 2026-09-08
- **Baseline:** `codex/poc-0.3.1-architecture` at `18971e4`
- **Purpose:** recover useful product reasoning from earlier branches without
  restoring obsolete implementation or contradicting the clean 0.3.1 contract.

## Executive finding

The current branch has the correct architectural direction: three capabilities,
four use cases, one Linux application core, ports and adapters, transactional
last-valid state, an isolated Windows bridge and executable G0-G6 gates.

Its weakness was not a missing component diagram. It was missing connective
material between the product contract and implementation:

- no repository entry point or recommended reading order;
- little rationale for capability ownership and dependency rules;
- no explicit state model, safe publication order or failure-containment model;
- gates stated as one-line outcomes instead of reproducible test protocols;
- no implementation sequence for building a thin end-to-end PoC;
- historical decisions mentioned in prose but not audited as retained or
  rejected.

This documentation set closes those gaps. It does not claim that the runtime
exists or that any gate has passed.

## Sources reviewed

| Source | What it contributes | What must not return |
| --- | --- | --- |
| Current branch, commits `83de65a`, `086abfb`, `18971e4` | Clean capability model, target tree, G0-G6, Windows isolation and renderable diagrams | Nothing; this is the authority being clarified |
| `codex/poc-archive` at `a7d1e6f` | Strong operational language, trust-boundary explanation, release gates, diagnostics and rollback thinking | Old capability commands, eight-opcode broker, dnsmasq, provider-specific topology and text output contract |
| `codex/restore-business-architecture` at `514ebd3` | Business-first taxonomy, honest health checks, explicit trust boundaries, recovery evidence and adapter vocabulary | The earlier `install/connect` application model, separate `ops` orchestrators and the old mesh/control topology |
| `main` at `322a5a1` | Detailed DNS/TLS negative tests, candidate publication order, failure isolation and PoC caveats | `init`, per-capability public commands, operator-owned WSL and implementation choices removed from the clean baseline |
| `legacy` at `805352d` | Detailed language for convergence, separation of intent/secrets/observed state, supply-chain gates, recovery and Windows runtime ownership | Legacy runtime, tray, schedulers, provider framework, generic privileged execution and broad product scope |

The repository also contains an uncommitted historical stash rooted at the
current `main` baseline. It was inspected only as supporting context; no content
was restored from it as an authority.

## Decisions retained

### Recover behavior, not old code

Earlier branches are useful for identifying properties that survived real
operation: persistent identity, deliberate trust installation, authenticated
health, narrow Windows mediation, idempotent apply and reboot recovery. They are
not a source tree to copy. New code must follow the target tree and current
contracts.

### One orchestration core

The prior repository accumulated parallel Rust applications and host scripts
that each owned part of convergence. The clean design makes `src/app` the only
owner of the four use cases. Adapters translate environment operations; scripts
and runtime assets do not become a second application layer.

### Capability names are product language

Access, Control and Compute describe outcomes. CoreDNS, WSL, systemd, Podman and
future release-selected components are implementation names. Vendor names may
appear in adapters, runtime assets, the release lock and technical evidence, but
not as domain concepts or operator-selected provider families.

### Windows isolates routine operation

The useful legacy property is that the normal operator session does not own the
WSL distribution, runtime secrets or container engine. The dedicated
`gnx-runtime` identity, `GNXRuntime` service and bounded pipe are retained. The
boundary does not claim protection from SYSTEM or local Administrators.

### Success means observed behavior

A written file, valid generated configuration, active process or successful
compile is intermediate evidence. `READY` requires the capability-specific live
probe defined by the PoC gate. This rule was one of the strongest parts of the
earlier documentation and is now made explicit throughout the current set.

## Decisions explicitly not recovered

- No old public command surface. The only public operations are `doctor`,
  `plan`, `apply` and `status`.
- No generic broker execution, argv passthrough, path selection or WSL distro
  selection.
- No parallel `ops/access`, `ops/control` or `ops/compute` applications.
- No tray, updater, scheduler, provider/plugin framework, workload catalog,
  Podman Machine or proprietary journal.
- No assertion that a historical provider, image or endpoint is part of 0.3.1
  unless it is deliberately pinned in the new release definition.
- No automatic client trust installation and no secret in configuration, argv,
  logs or acceptance evidence.
- No reuse of old `READY <payload>` text output; JSON plus the documented exit
  semantics is the current contract.

## Documentation authority

When documents appear to conflict, use this order:

1. `business-requirements.md` — product outcome and non-negotiable boundaries;
2. `architecture.md` and `windows-runtime.md` — structural and platform rules;
3. `poc.md` — executable acceptance;
4. `release.md` — candidate content and promotion;
5. `implementation-plan.md` — delivery order, which may evolve without changing
   the product contract;
6. `README.md` — orientation only.

Git history is below all current documents. A contradiction is resolved by an
explicit current change, not by silently choosing the older behavior.

## Open decisions before implementation freezes

The architecture deliberately does not yet name every runtime component. The
following decisions must be recorded in the new release definition or a short
ADR before their first adapter is merged:

| Decision | Deadline | Required proof |
| --- | --- | --- |
| Private transport implementation and enrollment flow | Before Access adapter | persistent identity, authorized reachability, stdin-only secret path |
| Compute service and authenticated health contract | Before Compute adapter | immutable artifact, persistent data map, non-public upstream, real authenticated probe |
| Control artifact and GNX root lifecycle | Before Control adapter | the target tree names a `Caddyfile`, but requirements do not yet formally pin Caddy; the release must resolve that mismatch and prove hostname verification, explicit routes, deliberate trust, private-key custody and recovery |
| Exact JSON schema and stable diagnostic codes | Milestone 0 | Linux/Windows fixtures, unknown-field/version behavior and exit mapping |
| Release signing mechanism and trusted verification key | Before Windows packaging | producer authenticity, every artifact digest and rollback to last valid release |
| Supported Linux/Windows test matrix | Before G0 automation | exact versions, prerequisites, reboot procedure and known exclusions |

An open decision is not permission for an adapter to leak its vocabulary into
the domain. It is a release choice behind an existing port.

## Corrections made by this audit

- Added a root README with status, contract, configuration boundary, reading
  order and an honest definition of done.
- Expanded the architecture with responsibility, dependency, state, publication,
  ownership, security and failure-containment rules.
- Expanded the PoC gates into test protocols with positive, negative and
  persistence evidence.
- Added an implementation plan that prioritizes contracts and vertical slices
  over recreating the historical tree.
- Strengthened release promotion and historical-reuse rules.

The result is sufficient to start implementation. It is intentionally not a
substitute for the executable evidence that implementation must produce.
