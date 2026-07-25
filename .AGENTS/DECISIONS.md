# Decisions — 0.1.15

1. The observed MSI 2203/path failure is addressed by copying each dependency into a GNX-owned stable cache before `msiexec`; global Package Cache ACLs are not modified.
2. WSL and Podman are ancillary payloads of closed `gnx-host-preflight` helper modes, not direct Burn MSI packages.
3. Dependency version, file name, exact size and SHA-256 remain pinned in `installer/dependencies.lock.json` and duplicated only as build-validated Rust constants.
4. The install journal is product-version scoped and limits a repeated phase to three attempts.
5. Existing compatible dependencies are post-validated and reused; incompatible MSI registrations stop the installation.
6. `gnx -v` and `gnx --version` use `CARGO_PKG_VERSION` and never open the Named Pipe.
7. `prepare-member` and `authorize-member` are controlled reconciler decisions, not public CLI commands or new remote endpoints.
8. `confirm-membership` uses one new typed runtime-agent operation and existing PVE cluster state; it does not accept arbitrary argv.
9. Topology discovery accepts any number of members but still fails closed when zero or multiple controllers are visible.
10. Protocol schema 2, persisted-state schema 2 and runtime generation `proxmox-cluster-v2` remain unchanged. Runtime payload version advances to 5 because the agent allowlist changes.
