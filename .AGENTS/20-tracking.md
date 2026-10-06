# Work tracking

## Agent host
- Owns: `apps/host-agent`, `apps/setup`, `crates`, `installer`, `payload/host`.
- Milestone: Windows service and minimal console Setup; secured named-pipe contract; versioned installer payload and upgrade preservation.

## Agent node
- Owns: `apps/web-app`, `payload/node`, `tests/node-runtime`.
- Milestone: external runtime payload, PWA, Quadlets, canonical `verify.run`; tests.

## Integration
- Commit and push after each verified milestone.
- Do not change shared contract without an explicit coordinator message.
- Current mission: next versioned update for a dedicated `ai` LXC with Voice, Oryx, OmniRoute, Continuity and the official NetBird BYOP proxy bound to the existing management/account; preserve shared private-services identities/data and existing PWA auth. Source/scope: docs/machine-provisioning.md. Continuity source is the operator-confirmed `C:\Users\mayas\continuity`, commit d5c8886; fixed per-instance scope requires a separate AI spool/prefix using the same verified image, not a second business writer. Latest inspection: business container Up, NetBird cluster list empty, 32 component tests pass and real-S3 test ignored; docs/validation/ai-services/README.md. No feature deployment, new installer or S3 release yet. `.AGENTS/30-pwa-auth.md` remains the auth authority. CLI-only privileged onboarding remains unchanged. Coordinator assigns disjoint paths and exclusively runs tests, builds, installs, and live acceptance; workers edit code/docs only.
- AI/BYOP partial milestone: added guest-local account-token/custom-domain enrollment, 13 isolated tests, and an in-memory-transport read-only runtime inspector. Existing Continuity health reports business generation 0; NetBird clusters/proxy-tokens APIs return authenticated 200/count 0. Caddy remains the HTTPS edge, not a replacement for the official proxy. Operator selected private `.gnx` with the existing CA and no public exposure; the external AI profile declares `proxy.gnx` and `*.ai.gnx`. No lifecycle integration, AI guest, new build/installer or S3 release yet; do not label source publication as a completed EXE update.
- AI format correction: operator rejects added Python, `.in` templates and parallel lifecycle scaffolding. Removed those prototypes and reverted their unfinished Rust/IPC hooks; retain the existing LXC/mesh lifecycle. External `profile.json` and six real pinned Quadlets pass 13 definition contracts and the Quadlet dry-run generator (6 containers, exit 0); existing static regressions and shell syntax pass. This is definition evidence only: no AI guest, new EXE, database manifest registration or live acceptance. Tailscale must use its own `gnx-ai` state; do not copy existing identity. Do not touch the dirty Voice checkout.
- Auth release v0.1.72: signed Pocket subject + explicit group fixes unverified-email callback 500. Exact installer/upgrade NodeReady, 53 runtime checks + payload_current pass; host/guest identity, CA, credentials and datastore directories preserved. 66 Rust, 16 isolated auth checks and 6 compiled MSI condition cases pass. Real login observed after hotfix; no destructive uninstall or full real Pocket logout/revocation acceptance. See docs/validation/v0.1.72/README.md.
- Auth milestone v0.1.71: installed/upgrade NodeReady, credentials preserved, anonymous boundary verified and isolated signed-OIDC group/logout/replay checks pass. Real Pocket passkey/group/logout acceptance is blocked on human interaction; never convert protocol tests into a personal acceptance claim. Proxmox ebtables/pending-changes warnings remain a separate limitation.
- Keep generated builds under `target/` and `build/`; publish immutable versioned files under `releases/` only after verification. No scratch reports in product directories.
- Report tested commands, commit SHA, and remaining blocker.
