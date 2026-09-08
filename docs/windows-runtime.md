# Windows installation and isolated Linux runtime — GNX 0.3.1

This document makes the Windows boundary precise. It is normative for the new
PoC and refines BR-W01 through BR-W10.

## Context recovered from Git history

Commit `062332d` introduced the useful boundary: `GNXRuntime`, the dedicated
`.\gnx-runtime` account, a local named-pipe broker and a WSL distribution owned by
that account. Commit `7e26018` completed the clean-clone release path that built a
GNX Linux binary and packaged it for streaming into WSL.

The new PoC retains those boundaries, not the old command list, provider choices
or runtime implementation. The allowlist is now exactly `plan`, `apply`, `status`
and `doctor`; the Linux application core defined by the 0.3.1 architecture remains
the only orchestrator.

## Installed trust boundaries

```mermaid
flowchart TB
    subgraph OperatorSession[Normal operator session]
        Operator[Installing operator SID]
        CLI[gnx.exe<br/>parse request + render JSON]
        Operator --> CLI
    end

    subgraph WindowsHost[Windows host]
        Pipe["\\.\pipe\GNX<br/>local-only + protected DACL<br/>bounded opcode protocol"]
        SCM["Service Control Manager"]
        Service["GNXRuntime<br/>gnx-service.exe"]
        Account[".\gnx-runtime<br/>service logon only"]
        State["C:\ProgramData\GNX<br/>protected ACL"]
        Public["C:\ProgramData\GNX\public<br/>public CA only"]
        Trust["Explicit elevated CA trust step"]

        CLI --> Pipe
        Pipe --> Service
        SCM --> Service
        Account --> Service
        Service --> State
        Public --> Trust
    end

    subgraph IsolatedWSL["WSL distribution GNX — owned by gnx-runtime"]
        LinuxGNX["/usr/local/bin/gnx<br/>product-built, verified"]
        Config["/etc/gnx/gnx.toml<br/>0600, no secrets"]
        Runtime["systemd + Podman<br/>Access / Control / Compute"]
        Secrets["/var/lib/gnx<br/>root-owned runtime state"]

        LinuxGNX --> Runtime
        Config --> LinuxGNX
        Runtime --> Secrets
    end

    Service -->|fixed argv + config/secret on stdin| LinuxGNX
    LinuxGNX -->|sanitized JSON + exit code| Service
    LinuxGNX -->|public CA certificate only| Public
```

The pipe DACL permits SYSTEM, Administrators and the exact installing operator
SID. Remote pipe clients are rejected. The operator SID can request allowlisted
operations but does not gain filesystem access to the WSL disk or runtime state.
SYSTEM and local Administrators remain trusted principals.

The service-account password is generated with a cryptographically secure random
source during installation or rotation, passed only to SCM while registering the
service, and zeroized from process memory afterward. It is never written into GNX
configuration, state, logs or evidence.

## Retained boundary and 0.3.1 hardening

| Historical boundary retained | 0.3.1 requirement |
| --- | --- |
| Dedicated `gnx-runtime` account and `GNXRuntime` service | Same identity boundary, with explicit rights/ACL acceptance checks |
| Local protected pipe with bounded frames | Four opcodes only: `PLAN`, `APPLY`, `STATUS`, `DOCTOR` |
| Linux bundle streamed into the isolated distro | Authenticated manifest, inner binary digest and last-valid rollback |
| Fixed WSL distro with interop disabled | Fixed `GNX` ownership plus executable post-install verification |
| Secret payload separate from configuration | Two-phase `ACTION_REQUIRED`, zeroization and negative-leak tests |

## Release build and installation

```mermaid
sequenceDiagram
    participant CI as Trusted build pipeline
    participant I as Elevated install.ps1
    participant SCM as Windows SCM
    participant S as GNXRuntime / gnx-runtime
    participant W as WSL GNX

    CI->>CI: test + lint + build Windows CLI/service
    CI->>CI: cross-build and test GNX Linux binary
    CI->>CI: assemble runtime assets and Linux bundle
    CI->>CI: hash artifacts and sign release manifest
    CI-->>I: authenticated release bundle

    I->>I: verify manifest signature and all digests
    I->>I: ensure machine-wide WSL engine
    I->>SCM: stop existing GNXRuntime for update
    I->>I: install gnx.exe and gnx-service.exe
    I->>I: create/reconcile gnx-runtime rights and private ACL
    I->>SCM: create/update automatic GNXRuntime as gnx-runtime
    I->>I: stage verified rootfs + Linux bundle in ProgramData
    I->>SCM: start GNXRuntime

    SCM->>S: launch using service credential held by SCM
    S->>W: import fixed GNX distro as gnx-runtime if absent
    S->>W: configure systemd and disable automount and interop
    S->>W: stream verified bundle to root-only temporary directory
    S->>W: verify and install /usr/local/bin/gnx + runtime assets
    S->>S: remove bootstrap copies after successful install
    S-->>I: broker readiness + installed Linux digest
    I->>I: compare installed digest and run doctor
```

The release must authenticate the manifest, for example through a signed
installer or a detached signature rooted in a key already trusted by the
installer. Hash files shipped beside unsigned artifacts are insufficient to
establish producer authenticity.

The build-time WSL distribution is only a compiler environment. It is never
copied into product configuration. The installed distribution is always the
fixed `GNX` runtime owned by `.\gnx-runtime`.

## Sensitive request flow

```mermaid
sequenceDiagram
    actor O as Operator
    participant C as gnx.exe
    participant B as GNXRuntime broker
    participant L as /usr/local/bin/gnx
    participant A as Specific Linux adapter

    O->>C: gnx apply
    C->>C: parse and validate gnx.toml
    C->>B: APPLY opcode + intent frame
    B->>L: fixed argv with intent on stdin
    L-->>B: ACTION_REQUIRED with secret kind
    B-->>C: sanitized ACTION_REQUIRED
    C->>O: hidden prompt for that secret
    O-->>C: secret
    C->>B: APPLY opcode + bounded secret frame
    B->>L: secret on stdin, never argv/environment
    L->>A: scoped secret on stdin
    A->>A: consume and persist only derived runtime state if required
    A-->>L: result without secret
    L-->>B: sanitized JSON + exit code
    B-->>C: bounded response
    C->>C: zero secret buffers
    C-->>O: JSON result
```

The first request carries no secret. Linux decides whether one is required, so
Windows does not collect credentials unnecessarily. Secret buffers are bounded
and zeroized after use. Enrollment material is not written to Windows. If a
Linux adapter requires a temporary file because an upstream program lacks stdin
support, it must create it on a root-only in-memory filesystem, pass the filename
through fixed internal argv, and remove it with failure-safe cleanup.

## Fixed broker contract

The broker protocol contains a version marker, one opcode, bounded intent and
secret lengths, and a bounded response. The only operation mapping is:

| Opcode | Linux action | Mutation |
| --- | --- | --- |
| `PLAN` | `gnx plan` | No |
| `APPLY` | `gnx apply` | Yes, reconciled |
| `STATUS` | `gnx status` | No |
| `DOCTOR` | `gnx doctor` | No |

There is no `exec`, shell string, argv passthrough, Windows path, WSL distribution
selector or provider selector. The service invokes the absolute product path and
the fixed `/etc/gnx/gnx.toml` path.

## Installation invariants

Installation or update reports `READY` only when all of these hold:

1. The release signature and every required digest are valid.
2. `.\gnx-runtime` has service-logon permission and the three deny-logon rights.
3. `GNXRuntime` is automatic, runs under `.\gnx-runtime` and has bounded restart
   actions.
4. The ProgramData ACL and pipe DACL match their exact principals.
5. WSL `GNX` belongs to `.\gnx-runtime`; systemd is enabled while automount and
   Windows interop are disabled.
6. `/usr/local/bin/gnx` is root-owned, not group/world-writable, and matches the
   authenticated release digest.
7. Bootstrap copies are absent after successful installation.
8. Broker ping, `gnx doctor` and Windows/Linux schema parity pass.

An update preserves the last valid Linux runtime until the candidate has passed
digest, installation and health checks. A failed candidate returns `FAILED` and
must not replace the last valid executable or configuration.
