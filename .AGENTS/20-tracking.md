# Work tracking

## Agent host
- Owns: `apps/host-agent`, `apps/setup`, `crates`, `installer`, `payload/host`.
- Milestone: Windows service and minimal console Setup; secured named-pipe contract; versioned installer payload and upgrade preservation.

## Agent node
- Owns: `apps/web-app`, `payload/node`, `tests/node-runtime`.
- Milestone: external runtime payload, PWA, Quadlets, canonical `verify.run`; tests.

## Current mission and authoritative state (2026-10-07)

- First correct agent behavior using `.AGENTS/40-config-driven-deployments.md`; generic deployment/upload design and slimming baseline: `docs/config-driven-platform.md`.
- No live hotfixes or scratch runners. Reusable protected Tailscale enrollment key is the approved target; the current implementation consumes the key and has a separate AI join. Do not claim the target is implemented.
- Candidate source is v0.1.78 with substantial uncommitted changes. Saved evidence/logs confirm installation exit 0 and an AI guest prepared with images, but no accepted AI services/HTTPS/login/inference. Later curl/cache source corrections are not in that EXE. Do not publish it as a completed AI release.
- Host-user builder `GnX-SlimFit` audited and unregistered; replacement `GnX-LinuxLab` is Debian 13/systemd/Podman 5.4.2, protected publisher credentials preserved, Quadlet dry-run passed. Net ~28.10 GiB freed; C: ~35.61 GiB free. Dedicated `gnxnodesvc` runtime `gnx-node`/LXC/data preserved. Full evidence: `docs/validation/deployment-paradigm/README.md`.
- Installed diagnostic found transport timeout/protocol/STUN failures while preserved projection remains NodeReady; temporary CLI unavailability recovered without service restart. Do not present that projection as new health acceptance. Resolve through source/build/install, not a runtime patch.
- Implementation workers: `gpt-5.6-luna`, effort high, disjoint paths. Coordinator alone builds/installs/verifies and cleans owned outputs. HF is the current source remote; existing S3 artifact destination still requires confirmed publication config.

## Integration
- Commit and push after each verified milestone.
- Do not change shared contract without an explicit coordinator message.

### Historical checkpoints (not current deployment claims)

- Prior mission: next versioned update for a dedicated `ai` LXC with Voice, Oryx, OmniRoute, Continuity and the official NetBird BYOP proxy bound to the existing management/account; preserve shared private-services identities/data and existing PWA auth. Source/scope: docs/machine-provisioning.md. Continuity source is the operator-confirmed `C:\Users\mayas\continuity`, commit d5c8886; fixed per-instance scope requires a separate AI spool/prefix using the same verified image, not a second business writer. Latest inspection: business container Up, NetBird cluster list empty, 32 component tests pass and real-S3 test ignored; docs/validation/ai-services/README.md. No feature deployment, new installer or S3 release yet. `.AGENTS/30-pwa-auth.md` remains the auth authority. CLI-only privileged onboarding remains unchanged. Coordinator assigns disjoint paths and exclusively runs tests, builds, installs, and live acceptance; workers edit code/docs only.
- AI/BYOP partial milestone: added guest-local account-token/custom-domain enrollment, 13 isolated tests, and an in-memory-transport read-only runtime inspector. Existing Continuity health reports business generation 0; NetBird clusters/proxy-tokens APIs return authenticated 200/count 0. Caddy remains the HTTPS edge, not a replacement for the official proxy. Operator selected private `.gnx` with the existing CA and no public exposure; the external AI profile declares `proxy.gnx` and `*.ai.gnx`. No lifecycle integration, AI guest, new build/installer or S3 release yet; do not label source publication as a completed EXE update.
- AI format correction: operator rejects added Python, `.in` templates and parallel lifecycle scaffolding. Removed those prototypes and reverted their unfinished Rust/IPC hooks; retain the existing LXC/mesh lifecycle. External `profile.json` and six real pinned Quadlets pass 13 definition contracts and the Quadlet dry-run generator (6 containers, exit 0); existing static regressions and shell syntax pass. This is definition evidence only: no AI guest, new EXE, database manifest registration or live acceptance. Tailscale must use its own `gnx-ai` state; do not copy existing identity. Do not touch the dirty Voice checkout.
- Auth release v0.1.72: signed Pocket subject + explicit group fixes unverified-email callback 500. Exact installer/upgrade NodeReady, 53 runtime checks + payload_current pass; host/guest identity, CA, credentials and datastore directories preserved. 66 Rust, 16 isolated auth checks and 6 compiled MSI condition cases pass. Real login observed after hotfix; no destructive uninstall or full real Pocket logout/revocation acceptance. See docs/validation/v0.1.72/README.md.
- Auth milestone v0.1.71: installed/upgrade NodeReady, credentials preserved, anonymous boundary verified and isolated signed-OIDC group/logout/replay checks pass. Real Pocket passkey/group/logout acceptance is blocked on human interaction; never convert protocol tests into a personal acceptance claim. Proxmox ebtables/pending-changes warnings remain a separate limitation.
- Keep generated builds under managed `target/` and `build/`; final sanitized evidence under `docs/validation/`. No agent-authored temporary scripts/clones/runners, even under build. Tool intermediates need ownership, budget and cleanup; retain immutable releases only after acceptance. Follow `.AGENTS/40-config-driven-deployments.md`.
- Report tested commands, commit SHA, and remaining blocker.
