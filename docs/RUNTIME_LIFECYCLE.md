# Runtime lifecycle and recovery

The 0.1.14 reconciler preserves this order:

1. validate the dedicated Windows service identity;
2. configure WSL and ensure the managed Podman Machine;
3. validate Fedora, KVM, TUN and FUSE;
4. apply and verify payload v4 and the runtime-agent handshake;
5. load protected configuration and persisted state;
6. enroll or resume Tailscale and resolve the persistent role;
7. prepare PVE identity and start the nested runtime;
8. apply the PVE credential and verify Tailscale Serve;
9. create/verify the controller cluster or resume/join the member;
10. persist READY and publish final status.

Recovery is convergence-based:

- compatible resources are verified and reused;
- payload writes are temporary, hash-checked and atomic;
- controller state at or beyond the cluster checkpoint triggers verification;
- member joining state resumes against the pinned controller;
- incompatible machine generation triggers controlled recreation with Tailscale-state preservation.
