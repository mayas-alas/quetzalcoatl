# Scope

- Product workspace is this repository root: `apps/`, `crates/`, `payload/`, and `installer/`.
- Architecture authority: `docs/architecture.md`.
- Setup is a minimal Rust CLI; no WebView2, local web UI, or browser credential bridge.
- No localhost HTTP control plane. Setup ↔ host agent uses the secured Windows Named Pipe.
- Release retention: only the latest validated `releases/v*/` contains notes, hashes and download links. Verified Burn (with embedded MSI) is published in the Hugging Face artifact repository `gnx-labs-mx/gnx-node-releases`, not duplicated in the quota-limited Space. Older versions retain Markdown only under `docs/releases/`; `CHANGELOG.md` indexes release history. After notes and live validation, use `installer/scripts/verify.ps1 -PublishRelease`; publish via the host user's authenticated WSL, never the dedicated runtime identity. Do not rewrite Git/LFS history.
- `https://app.gnx` is the unprivileged status/access PWA, never an onboarding client.
- Node credentials are entered interactively with echo disabled; never via argv, logs, URLs, or environment variables. The explicit GitHub-only PWA configuration uses an in-memory token against api.github.com; never persist it or bridge it to Windows.
- Preserve the dedicated identity, WSL distro, CA, and enrolled credentials during upgrades.
- Do not embed runtime config, Quadlets, gateway rules, or secrets in Rust.
- Workers use `gpt-5.6-luna`, effort `medium`. For this CLI migration, only the coordinator runs tests, builds, installers, and live verification.
