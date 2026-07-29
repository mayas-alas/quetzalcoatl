# Decisions — 0.1.17

1. Host resources are measured by a closed, compiled preflight operation.
2. `host-profile.json` is the single source of truth for WSL and Podman Machine sizing.
3. Approximately 6 GiB hosts are admitted only as laboratory runtime profiles, not certified cluster members.
4. Cluster-member certification begins at 12 GiB visible RAM with at least 4 machine CPUs and 6144 MiB machine RAM.
5. Missing or stale profiles require rerunning the bundle; the service does not silently reconstruct policy.
6. No new crate, public protocol, listener or port is justified for 0.1.17.
7. New-node role selection depends only on valid online controller presence. Existing members, candidates, stale peers and unrelated peers do not affect the decision.
8. If one or more online controllers exist, the node becomes a member and selects deterministically by stable Tailscale node ID. Multiple controllers are an operational anomaly but do not block 0.1.17 enrollment.
9. Persistent role state is committed only after the final Tailscale hostname is observed.
10. Tailscale Serve is applied only after PVE is ready on the fixed local backend.
11. Remote argv describes a closed operation. Variable JSON, configuration and secrets use bounded stdin.
12. Repository-owned multiline programs use interpreter stdin modes such as `sh -s` or `python3 -`; `sh -c`, `bash -c`, redirection and pipelines are prohibited.
13. Files are reserved for durable state or consumers that require a path. Durable writes must be GNX-owned, bounded and atomic.
14. The remote-execution exception set is empty. Any future exception requires a decision update, a narrow validator allowance, a dedicated test and an explicit removal condition.
