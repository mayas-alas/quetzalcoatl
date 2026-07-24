# Remote execution remediation

## Closed condition

- Rust uses the closed `RuntimeOperation` enum for every runtime-agent call.
- The shell agent has exact argument-count checks and no generic execution branch.
- Runtime and payload sources contain no `sh -c` or `bash -c` execution.
- Static probes remain stdin-fed `sh -s` programs.
- Dynamic values and secrets are not inserted into shell program text.
- Remote stdin, stdout, stderr and execution time are bounded.
- `ci/validate_remote_execution.py` passes.
