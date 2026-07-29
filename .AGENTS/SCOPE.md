# Scope and definition of done — 0.1.17

## In scope

- Inventory logical CPU, visible RAM and system-disk capacity during host preflight.
- Persist and validate one bounded host profile for `.wslconfig` and Podman Machine creation.
- Preserve installer dependency staging, reboot recovery and bounded retry behavior.
- Resolve a new node from online controller presence only: zero means controller; one or more means member.
- Allow any number of existing members without using member count as a decision input.
- Commit the final role only after Tailscale confirms the renamed local identity.
- Start and validate PVE before applying Tailscale Serve.
- Apply structured Serve configuration through bounded stdin rather than shell redirection.
- Harden validators, tests and documentation around the closed remote-execution contract.

## Out of scope

- A custom Bootstrapper Application or new installer UI.
- Free-form user resource overrides.
- New crates, applications, services, listeners or ports.
- New IPC commands, persisted-state schema or runtime payload version.
- Multi-cluster identity within one tailnet, controller failover, HA or QDevice.
- Controller-side enrollment API, arbitrary remote execution or generic repair commands.
- Renaming the transport API in 0.1.17; explicit `machine_exec*` primitives remain a later refactor.

## Definition of done

- Source validators pass from a clean extraction.
- Rust formatting, Clippy and workspace tests pass on the Windows build host.
- The observed 5864 MiB host selects a bounded laboratory profile without fixed 8192 MiB assumptions.
- A clean first node reaches controller readiness.
- A later node observes at least one online controller and enters the member path regardless of existing member count.
- Tailscale Serve is absent before PVE readiness and is successfully applied through stdin afterward.
- No direct remote argv contains shell-control syntax.
