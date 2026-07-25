# Target architecture for 0.2.x

0.1.15 establishes internal boundaries without creating packages prematurely. After upgrade, reboot and recovery behavior are proven, 0.2.x may separate executable applications from reusable libraries:

```text
apps/
├─ gnx-cli
├─ gnx-service
└─ gnx-host-preflight

crates/
├─ gnx-protocol
├─ gnx-domain
├─ gnx-state
├─ gnx-windows
├─ gnx-runtime
└─ gnx-diagnostics
```

A module becomes a crate only when its contract is stable, independently testable and required by more than one composition root or security boundary. Fine-grained crates for Podman, Tailscale or Proxmox are not planned until actual reuse justifies them.
