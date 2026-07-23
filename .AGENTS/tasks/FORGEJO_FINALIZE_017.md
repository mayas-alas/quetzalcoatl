# FORGEJO-FINALIZE 0.1.7 - Serialized, diagnosable reconciliation

## Objective

Produce Quetzalcoatl 0.1.7 as an in-place upgrade over 0.1.6. Correct the
remaining Forgejo v16 edit contract and make `gnx configure forgejo` reject
initial convergence races with explicit, secret-safe diagnostics.

## Required behavior

- Include the target local `login_name` explicitly in every Forgejo
  administrator edit request. Forgejo v16 maps omitted `login_name` to an empty
  value during `UpdateAuth`.
- Serialize Forgejo configuration against the service runtime convergence.
- Reject the operation unless the volatile status is exactly controller
  `READY` with Forgejo `ready`; persisted `state.json=READY` is not sufficient
  during service restart.
- Return stable host/guest stage identifiers on every payload failure without
  forwarding curl output, HTTP bodies, authorization data, usernames or
  passwords.
- Preserve and resume the pending credential written by 0.1.5/0.1.6.
- Assign new 0.1.7 MSI, package and Burn identities while preserving both
  upgrade codes.

## File ownership

- `Cargo.lock`
- `crates/*/Cargo.toml`
- `crates/gnx-service/src/main.rs`
- `crates/gnx-service/src/pipe.rs`
- `runtime/payload-v1/bin/gnx-forgejo-configure`
- `runtime/payload-v1/bin/gnx-forgejo-configure-guest`
- `runtime/payload-v1/manifest.json`
- `installer/package.wxs`
- `installer/bundle.wxs`
- `installer/build.ps1`
- `installer/wixext/Gnx.DeterministicBundle.wixext/**`
- `.AGENTS/DECISIONS.md`
- `.AGENTS/TRACKER.md`
- `.AGENTS/EVIDENCE.md`
- `.AGENTS/tasks/FORGEJO_FINALIZE_017.md`
- `docs/ARCHITECTURE.md`
- `docs/VALIDATION.md`
- `installer/docs/RUNBOOK.md`

## Verification

Run shell syntax checks for both Forgejo payloads, workspace format, Clippy,
workspace tests, two complete installer builds and direct byte comparison.
Live behavior remains unverified until the physical 0.1.6 controller upgrades
in place and retries the same pending credential.
