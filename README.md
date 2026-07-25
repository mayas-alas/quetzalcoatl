# Quetzalcoatl 0.1.13

Quetzalcoatl is a Windows 11 bootstrap and convergence service for one managed Fedora Podman Machine running a Tailscale-connected Proxmox VE cluster node.

Version 0.1.13 is a release-hygiene and CLI-contract update over the installed 0.1.12 MVP. It removes inactive legacy sources, preserves the four-crate/runtime architecture, exposes the complete runtime state through `gnx status`, and verifies that the exact `gnx.exe` built by Cargo is the one embedded in the MSI.

Start with:

- `docs/ARCHITECTURE.md`
- `docs/AUDIT_0.1.13.md`
- `docs/RUNTIME_LIFECYCLE.md`
- `docs/REMOTE_EXECUTION.md`
- `docs/VALIDATION.md`
- `installer/docs/RUNBOOK.md`
