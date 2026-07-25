# Controlled remote execution

The Windows service communicates with the managed Fedora machine through one typed runtime agent.

## Threat boundary

Podman Machine SSH ultimately receives a command string. Dynamic `sh -c`, caller-provided argv or generic execution would make correctness and security depend on shell quoting and would bypass the runtime allowlist.

## 0.1.15 rules

1. Runtime call sites select a `RuntimeOperation` enum variant.
2. Variants map to fixed argument vectors in `runtime/remote/operation.rs`.
3. Dynamic data is delivered only through bounded stdin.
4. The Fedora agent accepts an exact operation and argument shape.
5. Managed payload scripts use fixed subcommands and reject trailing input.
6. Output, timeout and cancellation limits remain enforced by the transport.
7. Secrets are zeroized in Rust and PVE join credentials are not accepted as argv.
8. Membership confirmation is the closed operation `pve-cluster-create confirm-member`; there is no generic PVE command.

## Forbidden patterns

```text
sh -c <dynamic>
bash -c <dynamic>
pve-exec <caller text>
RuntimeOperation::Arbitrary(...)
```

`ci/validate_remote_execution.py` and `ci/validate_cluster_contract.py` enforce the source-level operation contract. Runtime execution still requires a Windows/Fedora test.
