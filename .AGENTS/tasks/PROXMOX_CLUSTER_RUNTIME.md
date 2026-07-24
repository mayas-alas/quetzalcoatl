# Proxmox cluster runtime

Maintain the smallest complete runtime that prepares the dedicated Fedora Podman Machine, starts Proxmox VE with Tailscale connectivity, resolves a persistent controller/member role, creates or joins the cluster idempotently, and reports `READY` only after quorum is verified.

Acceptance requires an exact payload manifest, Rust checks, shell syntax checks, installer static contracts, and physical Windows/nested-virtualization cluster validation.
