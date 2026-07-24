# Scope and definition of done

## Product boundary

```text
Windows 11
  -> WSL2
  -> dedicated Fedora Podman Machine
  -> KVM, TUN and FUSE validation
  -> Proxmox VE container
  -> Tailscale identity and HTTPS Serve
  -> controller cluster creation or member join
  -> READY
```

## Functional acceptance

- A clean controller reaches `READY` with PVE healthy and quorate.
- A member discovers exactly one controller, joins idempotently and reaches `READY`.
- Restart resumes from persisted state without changing role or controller.
- Runtime status contains only platform components and cluster state.
- The Fedora payload contains only the files required for PVE, Tailscale and cluster operations.
