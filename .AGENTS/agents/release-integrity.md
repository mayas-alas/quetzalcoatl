# Agent: release integrity

## Ownership

- `runtime/payload/manifest.json`
- `installer/`, `ci/`, versions and WiX identities
- `.AGENTS/` and release evidence

## Objective

Integrate CLI hygiene and release-contract changes into one buildable 0.1.13 source release without changing the MVP runtime boundary.

## Invariants

- Keep `installer/build.ps1` as the single entry point.
- Preserve dependency pins, bundle chain order, MSI UpgradeCode and Burn upgrade family.
- Use new ProductCode, PackageCode and deterministic BundleId for 0.1.13.
- Validate source payload before Rust/MSI construction.
- Keep every payload file represented exactly once and hash-locked.
- Never claim Windows build, upgrade or physical cluster acceptance without captured output.
