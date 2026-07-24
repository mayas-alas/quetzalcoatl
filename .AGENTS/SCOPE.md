# Scope and definition of done — 0.1.12

## Product boundary

```text
Windows 11 service and CLI
  -> WSL2
  -> one dedicated Fedora Podman Machine
  -> KVM, TUN and FUSE validation
  -> one hash-locked runtime payload
  -> Proxmox VE plus Tailscale
  -> persistent controller or member role
  -> cluster verified or member joined
  -> READY
```

## Included

- preserve the four existing Rust crates and all CLI/Named Pipe contracts;
- extract reconciliation from the runtime module facade without changing stage order;
- replace free-form runtime-agent argv construction with a closed Rust operation enum;
- retain only static `sh -s` programs sent through stdin and reject `sh -c` command strings;
- bound remote stdin/stdout/stderr and terminate operations that exceed the transport timeout;
- remove shell-string execution from PVE credential configuration;
- separate runtime payload and Rust build verification from `installer/build.ps1`;
- preserve state schema, machine generation and controller/member semantics;
- generate coherent 0.1.12 MSI and Burn identities while retaining upgrade families.

## Excluded

- new Rust crates, GitHub Actions or hosted CI;
- OpenTofu, generalized OCI orchestration or enrollment HTTPS;
- tray UI, a second Windows service or a Fedora daemon/listener;
- state schema redesign, machine-generation migration or CLI changes;
- arbitrary command execution through the Fedora agent.

## Definition of done

Static source checks must pass locally. Release acceptance additionally requires Rust format, Clippy, workspace tests, the Windows installer build, upgrade from 0.1.11 and physical runtime verification.
