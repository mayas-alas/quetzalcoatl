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
- Current mission: Pocket ID login for the PWA, explicit group access and safe local logout; `.AGENTS/30-pwa-auth.md` defines the approved scope. CLI-only privileged onboarding remains unchanged. Coordinator assigns disjoint paths and exclusively runs tests, builds, installs, and live acceptance; workers edit code/docs only.
- Auth release v0.1.72: signed Pocket subject + explicit group fixes unverified-email callback 500. Exact installer/upgrade NodeReady, 53 runtime checks + payload_current pass; host/guest identity, CA, credentials and datastore directories preserved. 66 Rust, 16 isolated auth checks and 6 compiled MSI condition cases pass. Real login observed after hotfix; no destructive uninstall or full real Pocket logout/revocation acceptance. See docs/validation/v0.1.72/README.md.
- Auth milestone v0.1.71: installed/upgrade NodeReady, credentials preserved, anonymous boundary verified and isolated signed-OIDC group/logout/replay checks pass. Real Pocket passkey/group/logout acceptance is blocked on human interaction; never convert protocol tests into a personal acceptance claim. Proxmox ebtables/pending-changes warnings remain a separate limitation.
- Keep generated builds under `target/` and `build/`; publish immutable versioned files under `releases/` only after verification. No scratch reports in product directories.
- Report tested commands, commit SHA, and remaining blocker.
