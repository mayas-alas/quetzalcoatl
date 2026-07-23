# FORGEJO-RECOVERY 0.1.6 - Idempotent administrator promotion

## Objective

Produce Quetzalcoatl 0.1.6 as an in-place upgrade over 0.1.5. Correct
`gnx configure forgejo` after live evidence showed that Forgejo v16 ignores an
`admin` property during administrator-created user creation because that
property is not part of `CreateUserOption`.

## Required behavior

- Reconcile both a newly created user and the partially created non-admin user
  left by 0.1.5.
- Use the current managed administrator to create the requested account when
  absent, then promote/update it through the administrator edit endpoint.
- Verify the requested credential authenticates and has administrator status
  before committing the pending DPAPI credential.
- Preserve the 0.1.5 pending rotation so the operator can retry the same
  `gnx configure forgejo` values after upgrade.
- Emit bounded step diagnostics without usernames, passwords, authorization
  headers, request bodies, or response bodies.
- Keep all credential transport in the named pipe, DPAPI, stdin and protected
  ephemeral `/run` files.
- Assign new 0.1.6 MSI, package and Burn identities while preserving both
  upgrade codes.

## File ownership

- `Cargo.lock`
- `crates/*/Cargo.toml`
- `runtime/payload-v1/bin/gnx-forgejo-configure-guest`
- `runtime/payload-v1/manifest.json`
- `installer/package.wxs`
- `installer/bundle.wxs`
- `installer/build.ps1`
- `installer/wixext/Gnx.DeterministicBundle.wixext/**`
- `.AGENTS/DECISIONS.md`
- `.AGENTS/TRACKER.md`
- `.AGENTS/EVIDENCE.md`
- `.AGENTS/tasks/FORGEJO_RECOVERY_016.md`
- `docs/ARCHITECTURE.md`
- `docs/VALIDATION.md`
- `installer/docs/RUNBOOK.md`

## Verification

```powershell
& 'C:\Program Files\Git\bin\sh.exe' -n runtime/payload-v1/bin/gnx-forgejo-configure-guest
cargo fmt --all -- --check
cargo clippy -p gnx-service -- -D warnings
cargo test --workspace
powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer/build.ps1
git diff --check
```

Run the installer build twice and compare both MSI and Setup outputs directly.
Live recovery remains unverified until 0.1.6 upgrades the physical 0.1.5
controller and the operator retries the same pending rotation.
