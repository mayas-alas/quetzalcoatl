# Quetzalcoatl / GNX 0.3.1

GNX exposes three small capabilities:

- **Access** connects a private client identity to the node.
- **Control** publishes explicit `.gnx` names over HTTPS.
- **Compute** runs persistent infrastructure behind Control.

The runtime is intentionally one implementation, not a provider framework. The node configuration describes intent; release-specific components and immutable image references stay inside the release.

```text
client
  │
  ├─ private access
  │
  └─ https://compute.gnx
           │
        Control
           │
        Compute
```

## Public intent

Start from [`config/gnx.example.toml`](config/gnx.example.toml):

```toml
schema = 1
instance = "gnx"
node = "compute"

[network]
subnet = "10.90.0.0/24"
```

Routes are optional and only publish software whose lifecycle remains outside GNX.

## CLI

```text
gnx init
gnx plan
gnx apply
gnx status
gnx doctor
gnx access apply|status|enroll
gnx control apply|status
gnx compute apply|status
```

JSON on stdout is the command contract. Exit `0` is ready, `1` is failed, and `2` means operator action is required.

## Windows

The Windows binary is only a bridge into the dedicated Linux runtime named `GNX`; it is not a second runtime implementation. Build steps are in [`docs/build-windows.md`](docs/build-windows.md).

## Status

0.3.1 is a PoC source release. Architecture and acceptance are defined in [`docs/architecture.md`](docs/architecture.md) and [`docs/poc.md`](docs/poc.md). A successful compile alone does not certify the runtime gates.
