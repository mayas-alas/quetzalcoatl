# Integration contract

Mandatory change/acceptance workflow: `40-config-driven-deployments.md`.
The generic profile/upload/reusable-key contract in `docs/config-driven-platform.md`
is a target, not permission to bypass the installed operations or privilege boundary.

- State source: `%ProgramData%\GnX\Node\state\`.
- IPC operations are versioned in `crates/control-protocol/src/lib.rs`; onboarding uses GetProgress, Provision, JoinMesh, Retry, Cancel, with installed runtime/workload/diagnostic operations in that same allowlisted contract. No arbitrary shell or Deprovision API. Do not add an operation per application.
- CLI Setup submits only existing versioned operations; secrets use protected one-shot files referenced through the Named Pipe.
- Package installation finishes before interactive onboarding; silent installs never prompt. Upgrades must not silently re-enroll or reset passwords.
- Runtime commands: `install.run check|install`, then `verify.run`.
- Runtime emits sanitized JSON Lines to stdout and nonzero exit codes on failure.
- Image references are fixed by digest in their respective Quadlets.
- NODE_READY requires platform services plus local verification; external CA/Split-DNS is ACTION_REQUIRED. It does not by itself certify every workload. Report workload acceptance and observation errors separately; no fallback healthy/available after a failed diagnostic.
- Existing onboarding consumes the Tailscale auth key. The operator-approved reusable-key secret store and automatic new-sidecar enrollment require a versioned migration, tests, build/install and acceptance; never claim they already work or copy mesh identity state.
