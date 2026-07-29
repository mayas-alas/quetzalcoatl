# Quetzalcoatl 0.1.17 buildfix-03

## Purpose

Consolidate the remote-execution standard revealed by the Serve failure and correct stale 0.1.17 documentation that still described superseded topology behavior.

## Documentation corrections

- Defines argv as operation identity, stdin as variable-data transport and files as durable state.
- Documents the real 0.1.17 transport names without claiming future `machine_exec*` APIs already exist.
- Adds a mandatory review checklist covering execution layer, exact argv, stdin schema, durable files, secrets, limits, timeout and idempotency.
- Corrects controller/member documentation to the implemented rule: zero online controllers means controller; one or more means member; existing members do not affect selection.
- Corrects lifecycle documentation so PVE readiness precedes Serve application.
- Updates active agent scope, decisions, tracker and evidence to the current release rather than the original host-profile-only mission.

## Validator hardening

`ci/validate_remote_execution.py` now examines explicit argv arrays passed to `machine_stdin` and `machine_stdin_output`. It rejects shell-control syntax such as redirection, pipelines, chained commands and command substitution in addition to the existing `sh`/`bash -c` check.

## Runtime impact

None. No product executable, protocol, persisted-state schema, payload, installer identity, topology rule or network surface changed in buildfix-03.
