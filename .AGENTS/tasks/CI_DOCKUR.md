# CI-01 - Dockur GitHub Actions lane

## Objective

Turn the existing GitHub harness into a reproducible Dockur-on-Linux compatibility lane for the actual GNX installer/runtime. Do not use a Windows-native runner as a required signal.

## Working repository

The GitHub harness is `target/external/windows-rdp-tailscale`, not the primary GitLab checkout. Work from the assigned worktree and existing `codex/dockur-second-host` lineage.

## Required work

- Audit the existing workflows and reuse the pinned `dockurr/windows` image.
- Preserve or improve `/dev/kvm`, nested virtualization, TUN, memory, and disk preflight.
- Define three logical installation slots when runner capacity permits: controller, member 1, member 2. If a single hosted runner cannot safely run all three, encode a documented, deterministic staged strategy rather than pretending quorum was tested.
- Move installer lifecycle coverage into Dockur where technically possible.
- Expose noVNC/RDP only through a bounded, authenticated path and never print credentials or auth keys.
- Upload redacted diagnostics and machine-readable evidence.
- Mark DERP or RTT >= 5 ms as an explicit failure for the cluster network probe, while clearly classifying the overall run as compatibility evidence rather than physical-lab acceptance.
- Keep native `windows-latest` jobs out of required status checks; do not delete historical workflows unless the prompt explicitly authorizes it.

## File ownership

Only files inside the GitHub harness repository. Do not edit the primary Quetzalcoatl checkout.

## Verification

Validate workflow YAML and scripts locally where possible. Do not dispatch a workflow, push, or expose a session; the architect owns remote actions.
