# Runtime lifecycle and recovery

The 0.1.17 reconciler uses this order:

1. validate the dedicated Windows service identity;
2. load the persisted host profile and configure WSL;
3. ensure the managed Podman Machine with the selected CPU, RAM and disk;
4. validate Fedora and nested KVM;
5. apply and verify runtime payload version 5;
6. start Tailscale without publishing Serve;
7. validate local Tailscale readiness and observe online controllers;
8. resolve a new role or validate the persisted role;
9. rename Tailscale and confirm the final local identity before committing new role state;
10. configure local PVE identity and services;
11. create the controller cluster or execute the bounded member sequence;
12. wait for the fixed local PVE backend on port 8006;
13. generate and apply the fixed Serve JSON through stdin;
14. verify Serve status and publish final readiness.

## Controller

A new node that observes no valid online `gnx-controller-*` peer becomes the controller. Existing members, candidates, malformed peers and offline controllers do not affect this decision. The controller creates and verifies the `quetzalcoatl` PVE cluster before Serve is published.

## Member

A new node that observes one or more valid online controllers becomes a member. It selects the controller deterministically by stable Tailscale node ID; the number of existing members is not a limit.

```text
MEMBER_PREPARING
→ MEMBER_AUTHORIZING
→ MEMBER_JOINING
→ MEMBER_VERIFYING
→ MEMBER_CONFIRMING
→ READY
```

The persisted checkpoint remains compatible with state schema 2. On restart, the idempotent join inspects PVE state before deciding whether `pvecm add` is still required. A previously joined member is revalidated before returning to readiness.

## Upgrade

When valid persisted state exists, GNX preserves the existing role. It validates the local Tailscale identity and, for a member, the persisted controller. Discovery is not rerun merely because other members were added.

## Serve readiness

Tailscale connectivity, certificate availability and Serve readiness are separate states. HTTPS is configured only after PVE accepts the fixed local backend connection. Serve configuration is serialized in Rust and sent to `tailscale serve set-raw` over stdin; no remote file redirection participates in the operation.

## Installer recovery is separate

Dependency setup uses `C:\ProgramData\Quetzalcoatl\Installer\install-state.json`. Runtime state remains under the established product state location; installer recovery does not alter cluster-state schema.
