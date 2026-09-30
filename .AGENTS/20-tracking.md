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
- Current mission: CLI-only onboarding. Coordinator assigns disjoint paths and exclusively runs tests, builds, installs, and live acceptance; workers edit code/docs only.
- Keep generated builds under `target/` and `build/`; publish immutable versioned files under `releases/` only after verification. No scratch reports in product directories.
- Report tested commands, commit SHA, and remaining blocker.
