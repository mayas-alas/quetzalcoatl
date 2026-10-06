# Scope

- Product workspace is this repository root: `apps/`, `crates/`, `payload/`, and `installer/`.
- Architecture authority: `docs/architecture.md`. PWA auth authority: `.AGENTS/30-pwa-auth.md` (OAuth2 Proxy + Pocket ID + Valkey, explicit group, local logout; no NetBird migration).
- Setup is a minimal Rust CLI; no WebView2, local web UI, or browser credential bridge.
- No localhost HTTP control plane. Setup ↔ host agent uses the secured Windows Named Pipe.
- Release retention: only the latest validated local `releases/v*/` contains verified Burn (with embedded MSI), notes and hashes. Publish source/notes/hashes in the Hugging Face Space; prefer its release folder for the binary when quota allows, otherwise use the existing `gnx-labs-mx/gnx-node-releases` dataset and link its verified immutable revision. Do not duplicate the MSI; check quota without purging history. Keep binaries out of source Git and explicitly upload the artifact. Older versions retain Markdown only under `docs/releases/`; `CHANGELOG.md` indexes release history. After notes and live validation, use `installer/scripts/verify.ps1 -PublishRelease`; publish via the host user's authenticated WSL, never the dedicated runtime identity. Do not rewrite Git/LFS history.
- `https://app.gnx` is the unprivileged status/access PWA, never an onboarding client.
- Node credentials are entered interactively with echo disabled; never via argv, logs, URLs, or environment variables. The explicit GitHub-only PWA configuration uses an in-memory token against api.github.com; never persist it or bridge it to Windows.
- Preserve the dedicated identity, WSL distro, CA, and enrolled credentials during upgrades.
- Do not embed runtime config, Quadlets, gateway rules, or secrets in Rust.
- Workers use `gpt-5.6-luna`, effort `medium`. For this CLI migration, only the coordinator runs tests, builds, installers, and live verification.
