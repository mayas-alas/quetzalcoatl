# Quetzalcoatl operator runbook

## Purpose

The installer prepares WSL2, a dedicated Fedora Podman Machine, Proxmox VE, Tailscale and Proxmox cluster membership.

## Install and configure

1. Run `QuetzalcoatlSetup.exe` as an administrator.
2. Open an elevated terminal.
3. Run:

```powershell
gnx configure
```

Provide:

- the lowercase tailnet DNS suffix ending in `.ts.net`;
- a valid Tailscale auth key;
- a new PVE root password of 12–128 characters.

4. Check progress:

```powershell
gnx status
gnx status --json
```

A successful node reports `overall=ready`, `stage=READY`, healthy platform components and a joined/quorate cluster.

## Restart convergence

```powershell
gnx restart
```

The role and controller are loaded from protected persistent state. A restart does not repeat topology selection.

## Controller behavior

With no eligible peer, the first node becomes controller, receives a stable hostname derived from its Tailscale node ID, creates or verifies the PVE cluster and reaches `READY`.

## Member behavior

A later node must discover exactly one eligible controller with a direct Tailscale path. It persists that controller before joining, resumes an interrupted join, verifies quorum and reaches `READY`.

## Failure handling

Use `gnx status --json` and the Windows service log. Error messages are bounded and must not contain secrets. Failure categories cover Windows identity, WSL, Podman Machine, required devices, payload integrity, PVE health, Tailscale enrollment, topology, direct-path requirements, state mismatch and cluster join.

## Upgrade

The current state record is reduced to the cluster contract and the local cluster is verified before returning to `READY`.

## Uninstall

Normal uninstall removes the Windows product. Persistent PVE data is not deleted automatically.
