# Quetzalcoatl operator runbook — 0.1.17

## Install

1. Run `QuetzalcoatlSetup.exe` as an administrator.
2. Allow setup to reboot Windows when WSL features require it.
3. After resume, setup stages the pinned WSL and Podman MSIs under:

```text
C:\ProgramData\Quetzalcoatl\Installer\cache
```

4. On a dependency failure, retain:

```text
C:\ProgramData\Quetzalcoatl\Installer\install-state.json
C:\ProgramData\Quetzalcoatl\Installer\logs
```

Do not delete the stable cache before collecting evidence.

## CLI and configure

```powershell
gnx --version
gnx configure
```

Provide:

- the lowercase tailnet DNS suffix ending in `.ts.net`;
- a valid Tailscale auth key;
- a PVE root password of 12–128 characters.

Check progress:

```powershell
gnx status
gnx status --json
```

## Controller behavior

With no eligible GNX peers, the first node becomes controller, receives a stable hostname derived from its Tailscale identity, creates or verifies the PVE cluster and reaches `READY`.

## Member behavior

A later node must discover exactly one eligible controller. Any number of existing members may be visible. The member progresses through:

```text
MEMBER_PREPARING
MEMBER_AUTHORIZING
MEMBER_JOINING
MEMBER_VERIFYING
MEMBER_CONFIRMING
READY
```

`MEMBER_JOINING` preserves the existing idempotent `pvecm add`. Confirmation requires controller and member visibility in PVE cluster state.

## Restart convergence

```powershell
gnx restart
```

Role and controller identity are loaded from protected persistent state. A joined member is revalidated before returning to `READY`.

## Failure handling

Use `gnx status --json`, dependency MSI logs and the Windows service log. Error messages are bounded and must not contain secrets. A setup phase is attempted at most three times for the same product version before stopping with a resume-limit error.

## Upgrade

Upgrade from 0.1.14 preserves the MSI/Burn upgrade families. Validate protected configuration, managed machine identity and cluster membership after the upgrade.

## Uninstall

Normal uninstall removes the Windows product. Persistent PVE data and the diagnostic installer cache are not treated as implicit destructive cleanup; inspect them before manual removal.
