# Integration contract

- State source: `%ProgramData%\GnX\Node\state\`.
- IPC operations: GetProgress, Provision, JoinMesh, Retry, Cancel. No Deprovision API.
- CLI Setup submits only existing versioned operations; secrets use protected one-shot files referenced through the Named Pipe.
- Package installation finishes before interactive onboarding; silent installs never prompt. Upgrades must not silently re-enroll or reset passwords.
- Runtime commands: `install.run check|install`, then `verify.run`.
- Runtime emits sanitized JSON Lines to stdout and nonzero exit codes on failure.
- Image references are fixed by digest in their respective Quadlets.
- NODE_READY requires all four services plus local verification; external CA/Split-DNS is ACTION_REQUIRED.
