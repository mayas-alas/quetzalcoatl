# GNX

**Private infrastructure, one command surface.**

GNX is a Rust orchestrator for a small private infrastructure node. Linux runs the runtime natively; Windows exposes the same CLI while keeping the Linux runtime inside an isolated WSL2 identity.

## Execution model

### Linux

```text
gnx
└── systemd + Podman Quadlets
    ├── gnx-access      Tailscale
    ├── gnx-dns         dnsmasq
    ├── gnx-compute     Proxmox
    └── gnx-controller  Caddy
```

### Windows

```text
operator
└── gnx.exe
    └── \\.\pipe\GNX
        └── GNXRuntime service
            └── .\gnx-runtime
                └── WSL2: GNX
                    └── gnx + systemd + Podman
```

The Windows operator does not own the GNX distro or a Podman installation. `GNXRuntime` runs as the dedicated local account `gnx-runtime`; that account owns the WSL distro and launches every WSL operation.

WSL distro registration is per Windows user, so the `GNX` distro is registered in the dedicated account rather than in the operator's WSL context. The operator's normal `wsl -l` therefore does not list the GNX runtime. Local administrators and SYSTEM remain machine-level trust principals and can inspect or modify the host when elevated.

There is no Podman Machine, tray process, or second Windows implementation of the runtime.

## Capabilities

### Access

`gnx access` provisions the private access layer:

- Tailscale runs inside `gnx-access`.
- `gnx-dns` is a minimal dnsmasq instance for the `.gnx` Split DNS zone.
- Tailscale Services provide stable private service identities.
- Enrollment keys never enter `gnx.toml`, argv, Git, or logs.

On Windows, `gnx.exe` requests the auth key only when the isolated runtime reports that enrollment is required. The key crosses the local broker pipe and reaches Linux through `stdin`; it is not persisted by the Windows broker.

### Compute

`gnx compute` manages the local Proxmox service and verifies it through the API before returning success. The generated root password remains in the Linux state directory with restrictive permissions.

### Controller

`gnx controller` manages Caddy and the optional autonomous `.gnx` CA. The private root key never leaves Linux. When the CA exists, the broker may export only the public `root.crt` to `C:\ProgramData\GNX\public\root.crt` for the explicit Windows trust action.

## Windows isolation boundary

The broker accepts only the current GNX actions:

```text
access configure
access apply
access dns
compute apply
compute status
compute credentials
controller apply
controller status
```

It does not expose a generic shell or arbitrary `exec` path.

The named pipe rejects remote clients and its DACL is restricted to SYSTEM, local Administrators, and the SID of the operator that installed GNX. Runtime state under `C:\ProgramData\GNX` is owned by SYSTEM, Administrators, and `gnx-runtime`.

Inside the dedicated WSL distro, GNX disables Windows drive automount and Windows interop. Podman runs natively in that Linux environment.

## Configuration

GNX uses one declarative configuration file:

```text
config/gnx.example.toml
```

On Linux the active file is `/etc/gnx/gnx.toml`.

On Windows the operator edits `C:\Program Files\GNX\gnx.toml`. Each CLI request sends the validated configuration through the broker, which updates `/etc/gnx/gnx.toml` inside the isolated distro before invoking the requested action.

Secrets are not configuration fields.

## Basic workflow

```text
gnx compute apply
gnx controller apply
gnx access configure
gnx access dns
```

Health gates:

```text
gnx compute status
gnx controller status
gnx access dns
```

Operations preserve the stable output contract:

```text
READY <payload>
FAILED <LABEL>
```

## Build and installation

Windows release builds:

```powershell
.\packaging\windows\build.ps1 -Validate
```

The release contains three executable artifacts:

```text
gnx.exe          Windows CLI client
gnx-service.exe  isolated Windows broker/service
gnx              native Linux binary
```

`install-host.ps1` enables the machine-wide WSL engine when necessary, installs the two Windows binaries, creates the dedicated runtime identity, and starts `GNXRuntime`.

The runtime service imports its own `GNX` distro from the pinned Ubuntu 24.04 WSL rootfs declared in `packaging/windows/runtime.lock.json`. The installer verifies the rootfs SHA-256 before the service can consume it. The service then installs Podman and the GNX Linux bundle inside that distro.

Native Linux remains independent of the Windows path:

```bash
sudo ./install-linux.sh <bundle>
```

## Repository layout

```text
gnx/
├── src/
│   ├── windows/          Windows isolation + broker
│   └── bin/
│       └── gnx-service.rs
├── runtime/              Quadlets and runtime configuration
├── config/
├── packaging/
│   ├── linux/
│   └── windows/
├── tests/
└── docs/
```

## Documentation

- [`docs/arquitectura.md`](docs/arquitectura.md) — execution model, trust boundaries, Windows isolation, and runtime components.
- [`docs/operar.md`](docs/operar.md) — operational procedures.

## License

GNX is licensed under `AGPL-3.0-only`. Third-party components retain their respective licenses and attribution requirements.
