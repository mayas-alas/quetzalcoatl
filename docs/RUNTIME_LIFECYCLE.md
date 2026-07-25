# Runtime lifecycle and recovery

The 0.1.15 reconciler preserves the established host/runtime order:

1. validate the dedicated Windows service identity;
2. configure WSL and ensure the managed Podman Machine;
3. validate Fedora and nested KVM;
4. apply and verify runtime payload version 5;
5. enroll and stabilize Tailscale identity;
6. resolve or validate the persisted controller/member role;
7. configure local PVE identity and services;
8. create or join the PVE cluster;
9. verify readiness and publish status.

## Controller

A new tailnet with no GNX peers elects one controller. The controller creates and verifies the `quetzalcoatl` PVE cluster, then persists `READY`.

## Member

A node that discovers exactly one controller follows:

```text
MEMBER_PREPARING
→ MEMBER_AUTHORIZING
→ MEMBER_JOINING
→ MEMBER_VERIFYING
→ MEMBER_CONFIRMING
→ READY
```

The persisted checkpoint remains `Joining` through the intermediate phases, so the state schema does not change. On restart, the idempotent join inspects PVE state before deciding whether `pvecm add` is still required. A previously `Joined` member is revalidated before returning to `READY`.

## Multiple members

Discovery no longer rejects a topology based on member count. It still fails closed if peers exist without an identifiable controller or if more than one controller is visible.

## Installer recovery is separate

Dependency setup uses `C:\ProgramData\Quetzalcoatl\Installer\install-state.json`. Runtime state remains under the established product state location; installer recovery does not alter cluster-state schema.
