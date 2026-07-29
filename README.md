# Quetzalcoatl 0.1.17

Quetzalcoatl is a Windows-managed MVP that provisions and reconciles a Fedora Podman Machine containing the Tailscale and Proxmox runtime used by controller/member nodes.

Version 0.1.17 combines three installed-MVP corrections:

- dynamic host CPU, RAM and disk selection shared by `.wslconfig` and Podman Machine;
- lean role discovery based only on online controller presence;
- PVE-before-Serve ordering with structured Serve JSON delivered through stdin.

The workspace remains at four Cargo packages. Protocol schema 2, persisted-state schema 2, runtime payload 5 and runtime generation `proxmox-cluster-v2` are unchanged.

## Host profile

The installer records detected and selected resources at:

```text
C:\ProgramData\Quetzalcoatl\Installer\host-profile.json
```

A host with approximately 6 GiB visible RAM is intentionally classified as a laboratory profile. It may exercise installation and runtime creation, but it is not certified as a complete Proxmox cluster member.

## Lean topology

```text
zero online gnx-controller-* peers → controller
one or more online controllers     → member
existing member count              → ignored
```

Upgrades with valid persisted state preserve their existing role.

## Remote execution

Quetzalcoatl treats remote argv, stdin and files as separate contracts:

```text
argv  = closed operation
stdin = bounded variable data
file  = durable GNX-owned state
```

Dynamic shell execution, redirection and arbitrary remote commands are prohibited. See `docs/REMOTE_EXECUTION.md`.

## CLI

```powershell
gnx -v
gnx --version
gnx status
gnx status --json
gnx configure
gnx restart
```

## Build and validation

```powershell
python .\ci\validate_repository.py
python .\ci\validate_runtime.py
python .\ci\validate_remote_execution.py
python .\ci\validate_cli_contract.py
python .\ci\validate_release_contract.py
python .\ci\validate_installer_resume.py
python .\ci\validate_cluster_contract.py
python .\ci\validate_host_profile.py

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
.\installer\build.ps1
```

Start with `docs/README.md`, `docs/RUNTIME_LIFECYCLE.md`, `docs/REMOTE_EXECUTION.md` and `docs/VALIDATION.md`.
