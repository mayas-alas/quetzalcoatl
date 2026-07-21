# PVE-01 - Secure, resumable Proxmox member join

## Objective

Implement the guest-side payload contract for a member to join the existing controller cluster securely and idempotently.

## Required work

- Add a dedicated member join entrypoint or narrowly extend the existing cluster scripts.
- Validate API/SSH/Corosync connectivity, names, clock, MTU, direct Tailscale route, and required ports before `pvecm add`.
- Use the controller Tailscale IP for the target and the member Tailscale IP for `--link0`.
- Treat an already joined node as verify-and-continue.
- Make controller unavailability and incomplete joins resumable; never create a second cluster or change controller.
- Accept secrets only through stdin or an ephemeral `/run` file with mode `0600`; guarantee cleanup on success and failure.
- Add explicit member-side denial for OpenTofu/controller workload entrypoints where the payload can enforce it.
- Add shell-level/static tests or a deterministic test harness that does not require a live cluster.

## Non-goals

- No Rust role-discovery changes.
- No general cluster scaling.
- No Garage/Forgejo features.
- No CI workflow or documentation edits.

## File ownership

- `runtime/payload-v1/bin/`
- `runtime/payload-v1/systemd/` only if required
- `runtime/payload-v1/manifest.json` only for new payload files
