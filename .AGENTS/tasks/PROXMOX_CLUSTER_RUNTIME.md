# Proxmox cluster runtime

Maintain the smallest complete runtime that prepares the dedicated Fedora Podman Machine, installs the exact payload, verifies the on-demand runtime agent, starts Proxmox VE with Tailscale connectivity, resolves a persistent controller/member role, creates or joins the cluster idempotently, and reports `READY` only after quorum is verified.

The runtime agent is a constrained dispatcher, not a daemon or general remote shell. Pre-payload machine bootstrap remains on the existing SSH process transport.
