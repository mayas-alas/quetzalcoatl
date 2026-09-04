# GNX

**Private infrastructure, one command surface.**

GNX is a Rust orchestrator that turns a Linux host — or Windows through WSL2 — into a private infrastructure node with reproducible networking, compute, and HTTPS services.

The product keeps one execution model across both platforms:

- **Linux:** `gnx` runs natively.
- **Windows:** `gnx.exe` acts as a thin bridge and delegates Linux operations to `gnx` inside WSL2.
- **Runtime:** systemd + Podman Quadlets manage the services installed by GNX.

## What GNX provides

### Private access

`gnx access` provisions the private access layer using Tailscale and Pi-hole:

- Tailscale runs inside the managed `gnx-access` container.
- Split DNS for the `.gnx` zone is served by `gnx-dns`.
- Services can be exposed privately through Tailscale Services.
- Enrollment secrets are entered interactively and are never stored in `gnx.toml`.

### Compute

`gnx compute` manages the local compute service:

- Proxmox runs as a managed Podman Quadlet.
- The root password is generated locally from kernel entropy.
- GNX verifies service health through the Proxmox API before returning success.
- Credentials remain in root-owned state with restrictive permissions.

### Controller

`gnx controller` provides the HTTP/TLS entry point:

- Caddy proxies requests to the compute service.
- The primary private TLS path is provided through Tailscale.
- An autonomous `.gnx` CA is available as an explicit, optional capability.
- GNX never installs that CA into a client trust store automatically.

## Execution model

```text
Linux host
└── gnx
    └── systemd + Podman Quadlets
        ├── gnx-access
        ├── gnx-dns
        ├── gnx-compute
        └── gnx-controller

Windows host
└── gnx.exe
    └── WSL2
        └── gnx
            └── same Linux runtime
```

Windows does not maintain a second implementation of the runtime. The Windows binary validates and forwards commands to the Linux binary inside the configured WSL2 distribution.

## Basic workflow

```text
gnx compute apply
gnx controller apply
gnx access configure
gnx access dns
```

Health and verification commands use the same gates as installation:

```text
gnx compute status
gnx controller status
gnx access dns
```

Each operation finishes with either:

```text
READY <payload>
```

or:

```text
FAILED <LABEL>
```

## Configuration

GNX uses a single declarative configuration file:

```text
config/gnx.toml
```

Copy it to `gnx.toml` and adjust the deployment values for your environment.

Secrets do not belong in the configuration file. Enrollment keys and generated credentials use dedicated runtime paths and permission checks.

## Build and packaging

Windows release builds are produced with:

```powershell
.\packaging\windows\build.ps1
```

The release process builds and validates both artifacts:

```text
gnx.exe    Windows bridge
gnx        Linux native binary
```

Linux installation is provided through:

```bash
sudo ./install-linux.sh <bundle>
```

The Linux installer verifies the release checksum and installs `gnx` into `/usr/local/bin`.

## Repository layout

```text
gnx/
├── src/                 Rust orchestrator
├── runtime/             Quadlets and runtime assets
├── config/              example configuration
├── packaging/
│   ├── linux/
│   └── windows/
├── tests/               contract tests
├── docs/
│   ├── arquitectura.md
│   └── operar.md
├── Cargo.toml
└── AGENTS.md
```

## Documentation

- [`docs/arquitectura.md`](docs/arquitectura.md) — execution model, trust boundaries and component architecture.
- [`docs/operar.md`](docs/operar.md) — operational procedures and recovery workflow.

## License

GNX is licensed under `AGPL-3.0-only`. Third-party components retain their respective licenses and attribution requirements.
