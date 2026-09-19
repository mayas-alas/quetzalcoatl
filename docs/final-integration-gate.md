# 0.3.1 final integration gate

Date: 2026-09-18  
Worktree: `final-integration-gate`  
Commit base: `9c6334a`

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
| `cargo check --all-targets` | BLOCKED: active MSVC toolchain has no `link.exe` |
| `cargo test --all-targets` | BLOCKED: active MSVC toolchain has no `link.exe` |
| `cargo build --all-targets` | BLOCKED: active MSVC toolchain has no `link.exe` |
| GNU-target cargo test attempt | BLOCKED: GNU toolchain has no `dlltool.exe` |

The existing focused setup CLI tests cover strict bundle argument parsing,
secret-free default output, and `--json-progress` started/completed output;
source inspection confirms `--apply`, `--recover`, and `--rollback` are strict
finite dispatches and never emit `READY` without host verification. Native
Windows elevation, SCM/service identity, WSL import/reboot recovery, Podman,
DNS, TLS, and runtime acceptance remain unexecuted host acceptance gates.
No host acceptance is claimed here.

The ignored `target/` directory produced by failed local builds was removed
before commit; no generated artifacts or secrets are included.
