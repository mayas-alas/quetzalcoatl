# Delivery tracker — 0.1.17

- [x] Preserve 0.1.15 installer recovery and member checkpoints.
- [x] Add closed host inventory and one persisted resource profile.
- [x] Remove fixed WSL and Podman resource values.
- [x] Simplify new-node discovery to online controller presence.
- [x] Ignore member count and non-controller peer noise during role selection.
- [x] Commit role state after Tailscale rename verification.
- [x] Defer Serve activation until PVE readiness.
- [x] Replace Serve shell redirection with structured stdin.
- [x] Formalize argv/stdin/file execution policy and review checklist.
- [x] Strengthen source validation against multiline `sh -c` and shell syntax in remote argv.
- [ ] Certify Cargo formatting, Clippy and workspace tests on Windows.
- [ ] Build WiX bundle on Windows.
- [ ] Repeat clean Dockur controller installation.
- [ ] Repeat member installation against the ready controller.
