# Architecture decisions

## D-001 — Product completion is cluster READY
Convergence completes only after controller cluster verification or member join.

## D-002 — Four crates remain the MVP boundary
0.1.13 adds no crate and changes no state schema or machine generation.

## D-003 — CLI is a stable product boundary
The supported command surface remains `status`, `configure` and `restart`. Human and JSON status views represent the same complete `StatusResponse` contract.

## D-004 — CLI/service schema mismatch is an error
The CLI validates protocol schema version on status and operation responses so a partial or incoherent upgrade fails explicitly.

## D-005 — The installer owns CLI integrity
The MSI installs one keyed `gnx.exe`, adds `[INSTALLFOLDER]` to the system PATH and verifies the extracted CLI hash against the freshly built artifact.

## D-006 — One exact runtime payload
`runtime/payload` remains the only source payload; payload version stays at 4.

## D-007 — Fedora execution remains on-demand and typed
The runtime agent has no listener or generic exec operation. Fixed shell programs may use stdin-fed `sh -s`; dynamic shell command strings remain forbidden.

## D-008 — Build entry point remains stable
`installer/build.ps1` remains the single artifact-build entry point.
