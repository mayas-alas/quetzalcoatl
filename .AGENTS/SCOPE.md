# Scope and definition of done — 0.1.13

## Included

- preserve the four existing Rust crates and the `status`, `configure`, `restart` CLI surface;
- remove inactive legacy source files and closed release records from active scope;
- expose every `StatusResponse` field in human `gnx status` output;
- reject CLI/service protocol schema mismatches;
- verify `gnx.exe` source identity, MSI PATH registration and extracted binary hash;
- preserve runtime payload v4, state schema, machine generation and controller/member behavior;
- generate coherent 0.1.13 MSI and Burn identities while retaining upgrade families.

## Excluded

- new CLI commands, crates, services, daemon/listener or state migration;
- GitHub Actions, OpenTofu, tray UI or generalized orchestration;
- runtime behavior changes unrelated to source hygiene.

## Definition of done

All five static validators, Rust format, Clippy, workspace tests and the Windows installer build pass. Upgrade acceptance requires installing 0.1.13 over the existing 0.1.12 installation and confirming `gnx status`, configuration preservation and runtime readiness.
