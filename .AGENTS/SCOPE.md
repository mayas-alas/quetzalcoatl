# 0.2 platform foundation stabilization scope

## Outcome

Extend the READY Proxmox controller with a reproducible platform foundation
without returning service-specific infrastructure to Rust. Deliver one
`QuetzalcoatlSetup.exe` that installs, upgrades and repairs the platform, plus a
physical path from a Forgejo template repository to an OCI image, an isolated
LXC workload and a private Tailscale HTTPS URL.

## Included

| ID | Outcome | Acceptance |
|---|---|---|
| PCT | Bounded platform contract | Rust authorizes and reports generic platform operations; only the explicit `ADM` contract may name Forgejo or its fixed bootstrap identity. |
| BND | Locked platform bundle | One signed and hashed bundle contains fixed OpenTofu roots, closed LXC operations, Compose definitions, policies and the Forgejo template seed. |
| RUN | Closed platform runner | The controller is the sole IaC writer; OpenTofu owns resources and GNX streams fixed scripts through bounded stdin. |
| FND | Reproducible foundation | Independent host convergence installs Docker in every LXC before Garage, Forgejo and the dedicated runner follow their real dependencies. |
| STO | Isolated object storage | One Garage deployment exposes separate least-privilege buckets/keys for OpenTofu state and Forgejo objects; backend locking semantics are physically verified. |
| FRG | Sovereign repository entry | Forgejo contains a GNX Labs organization and one repository marked as a template; no example service source lives in Quetzalcoatl. |
| OCI | Isolated build path | A dedicated runner validates a repository, builds an OCI image, publishes it by digest and never receives PVE credentials. |
| SVC | Generic workload path | A signed release declaration drives one fixed OpenTofu service root, one state key, one LXC and one bounded GNX deployment. |
| NET | Private service exposure | `gnx configure platform` stores one separate DPAPI-protected service-enrollment key; Tailscale is managed at LXC level and the application reaches a private HTTPS URL after a health gate. |
| ADM | Closed Forgejo administration | Elevated local administrators can verify or atomically rotate the fixed bootstrap credential without placing it in argv, environment, state or logs. |
| REC | Safe recovery | Repair uses the installed locked bundle and foundation state; it does not recreate healthy resources, rotate identities or destroy durable data. |
| ARP | Sole maintenance entry | One visible Setup registration owns install, upgrade, repair and uninstall; internal packages remain hidden. |
| SUP | Verifiable supply chain | Rust binaries, MSI, Burn, platform bundle, OCI images and service releases have explicit digest/signature gates. |
| REL | Integrated release | Source, version, installer, upgrade, repair, status and physical foundation evidence agree on 0.2.40. |

## Preserved invariants

- The existing schema-2 core state and schema-1 host profile remain readable and
  authoritative for host identity, topology and cluster recovery.
- Platform state is separate and cannot change the controller/member decision.
- Closed remote argv; variable data uses bounded stdin.
- Controller/member role derives only from validated online topology.
- PVE readiness precedes platform reconciliation.
- No localhost UI, Windows listener, new Windows product port or Tauri runtime.
- No PVE credentials enter Forgejo, its registry, Actions or a runner.
- No service repository supplies HCL, providers, provisioners or remote commands.
- The node enrollment key and `installer-inputs.bin` are not reused as the platform
  secret. Platform enrollment uses schema-1 `platform-inputs.bin`, separate DPAPI
  entropy and `tag:quetzalcoatl-service`.
- Tailscale enrollment credentials are prohibited in repositories, Forgejo Actions
  secrets, Compose, `.env`, OCI images, logs, argv and OpenTofu state.
- Images use immutable digests; mutable tags are prohibited.
- `installer/build.ps1` remains the release entry point.

## Explicit contract amendments

- Runtime generation becomes `proxmox-platform`; payload contract becomes `6`.
- Protocol may add bounded platform operations while preserving reads of the
  current core status and state.
- Fixed repository-owned scripts are permitted only through the closed
  `pct exec <known-vmid> -- /bin/sh -s` operation. OpenTofu provisioners,
  caller-provided programs, arbitrary remote argv and shell-string execution
  remain prohibited.
- New listeners remain prohibited. Release discovery uses an explicit operation
  or bounded outbound polling.
- The product contract may name the single fixed `gnx-admin` bootstrap identity
  only for elevated `show` and confirmation-gated `reset` operations. It may not
  become a general Forgejo API or caller-selected account surface.

## Exclusions

- Hosted CI on another Windows host.
- Komodo, Kubernetes and Talos.
- Public Internet exposure of platform or workload services.
- Multiple templates, a general marketplace or arbitrary user-supplied Compose.
- Runner access to PVE, the OpenTofu state bucket or Windows service secrets.
- Destructive uninstall of platform data, LXC workloads, Garage objects or
  Forgejo repositories.

## Priority matrix

`score = impact × probability + urgency + necessity`, each factor 1–5.

| Risk | I | P | U | N | Score | Priority | Gate |
|---|---:|---:|---:|---:|---:|---|---|
| CI runner reaches PVE authority | 5 | 4 | 5 | 5 | 30 | P0 | OCI |
| Foundation cannot recover its own backend | 5 | 4 | 5 | 5 | 30 | P0 | STO/REC |
| Service repository injects execution | 5 | 4 | 5 | 5 | 30 | P0 | BND/SVC |
| Shared service state increases blast radius | 5 | 3 | 4 | 5 | 24 | P1 | SVC |
| Mutable image or bundle is deployed | 5 | 3 | 4 | 5 | 24 | P1 | SUP |
| Tailscale credentials leak into Compose | 5 | 3 | 4 | 5 | 24 | P1 | NET |

## Definition of done

All included rows are `done`; `tools/check.ps1` passes; the development-signed
0.2.40 Setup upgrades the installed 0.2.39 controller and repair reconverges the
same identities. Garage survives restart and passes S3 state/lock probes.
Forgejo opens over Tailscale, offers the injected service template and its
dedicated runner builds a repository created from that template. The controller
deploys the resulting immutable image to a distinct LXC and the private HTTPS
service URL passes its health probe. No PVE or Tailscale credential appears in
the runner, repository, Compose, logs, argv or OpenTofu state.
