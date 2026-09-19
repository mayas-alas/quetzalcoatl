# 0.3.1 final integration gate

Date: 2026-09-18
Worktree: `main-promotion`
Commit base: `origin/main` (`2612db0`) plus `integration/gnx-0.3.1-functional`

This report records the clean merge of the completed functional slice into a
main-based worktree. The user's `rama-mvp` worktree was not touched.

## Changes made

- Normalized all shell scripts to LF and added `.gitattributes` protection so
  Bash does not receive CRLF line endings from a Windows checkout.
- Changed generated Linux units and Access Quadlet units from unbounded
  `Restart=always` / `StartLimitIntervalSec=0` to `Restart=on-failure` with a
  five-failure burst in a 300-second window.
- Rollback now rejects reparse points/symlinks before removing any target path.
- Added an architecture regression test for bounded restart policies.

## Evidence

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| WSL `bash -n` over every `*.sh` | PASS |
| PowerShell AST parse over every `*.ps1`/`*.psm1` | PASS |
| `git diff --check` | PASS |
| Restart-policy scan for `Restart=always` or `StartLimitIntervalSec=0` | PASS |
| `cargo test --locked` | BLOCKED: active MSVC toolchain has no `link.exe` |
| PowerShell AST parse over every `*.ps1`/`*.psm1` | PASS (Windows PowerShell) |

The existing focused setup CLI tests cover strict bundle argument parsing,
secret-free default output, and `--json-progress` started/completed output;
source inspection confirms `--apply`, `--recover`, and `--rollback` are strict
finite dispatches and never emit `READY` without host verification. Native
Windows elevation, SCM/service identity, WSL import/reboot recovery, Podman,
DNS, TLS, and runtime acceptance remain unexecuted host acceptance gates.
No host acceptance is claimed here.

The ignored `target/` directory produced by the failed local build was removed
before commit; no generated artifacts or secrets are included. Missing MSVC,
elevated Windows, WSL and Podman acceptance is recorded as a blocker, and no
host `READY` claim is made.
