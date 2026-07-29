# Host resource profile — 0.1.17

## Purpose

The installer must not assume that every Windows host can provide six CPUs, 8 GiB of memory and a 100 GiB Podman disk. `gnx-host-preflight` now measures resources before Windows features or runtime creation and writes one product-version-scoped profile.

## Inventory

The closed PowerShell inventory reads:

- `Win32_ComputerSystem.NumberOfLogicalProcessors`;
- `Win32_ComputerSystem.TotalPhysicalMemory`;
- the total and free capacity of `%SystemDrive%` through `Win32_LogicalDisk`.

No path, command or script is supplied by the caller. The script is compiled into the preflight binary.

## Persisted contract

```text
C:\ProgramData\Quetzalcoatl\Installer\host-profile.json
```

The file contains detected CPU, RAM and disk values, the selected Podman profile, whether the host is runtime-capable and whether it meets the initial cluster-member certification threshold.

The write uses `host-profile.json.next`, flushes and synchronizes it, and then activates the completed file. A profile from another product version or schema is rejected by `gnx-service`.

## Resource policy

- Fewer than 4 logical CPUs, fewer than 4096 MiB RAM, fewer than 2048 MiB assignable RAM, or fewer than 40 GiB assignable disk: `install-only` and the bundle stops before runtime installation.
- 4096–8191 MiB RAM with sufficient CPU/disk: `lab`.
- 8192–12287 MiB RAM with sufficient CPU/disk: `runtime`.
- At least 12288 MiB RAM, at least 4 machine CPUs and at least 6144 MiB machine RAM: `cluster-member`.

The selected machine values remain bounded:

```text
CPU:    1–6
RAM:    2048–8192 MiB
Disk:   40–100 GiB
```

For the observed Dockur host with 5864 MiB and 4 logical CPUs, the expected selection is:

```text
capability:          lab
machine_cpus:        2
machine_memory_mib:  2560
```

Disk is calculated from the actual free capacity after reserving 20 GiB for Windows.

## Runtime application

`gnx-service` loads the profile before WSL preparation. It generates the service identity's managed `.wslconfig` from the selected CPU and RAM, and passes the same CPU, RAM and disk values to:

```text
podman machine init --cpus ... --memory ... --disk-size ...
```

Fixed `MACHINE_CPUS`, `MACHINE_MEMORY_MIB`, `MACHINE_DISK_GIB` and fixed `memory=8GB` configuration are no longer permitted.

## Limitations

A valid `lab` profile proves only that the host can attempt the managed runtime. It does not certify nested KVM, Proxmox readiness, Corosync or member join. Those remain runtime/e2e gates.
