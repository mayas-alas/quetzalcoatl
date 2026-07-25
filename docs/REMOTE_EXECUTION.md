# Remote execution contract

## Threat addressed

Podman Machine SSH ultimately receives a command string. Passing dynamic data through `sh -c` would make correctness and security depend on shell quoting.

## 0.1.13 rules

1. Runtime-agent call sites select a `RuntimeOperation` enum variant.
2. Enum variants map to fixed argument vectors in one file.
3. The Fedora agent validates operation names and exact argument counts.
4. The agent dispatches only to fixed `/usr/libexec/quetzalcoatl` paths.
5. No agent operation accepts an arbitrary command, executable path or shell program.
6. Multi-line bootstrap/probe programs are fixed source constants sent to `sh -s` through stdin.
7. Secrets and external values travel through stdin, argv with prior validation, or root-only ephemeral files.
8. Remote stdin is limited to 8 MiB, each output stream to 1 MiB, and one operation to 15 minutes.

## Allowed flow

```text
Rust domain module
  -> RuntimeOperation
  -> typed client
  -> bounded Podman Machine transport
  -> gnx-runtime-agent
  -> fixed helper
```

## Forbidden flow

```text
external value
  -> formatted shell string
  -> sh -c / bash -c
```

Run `python .\ci\validate_remote_execution.py` to enforce the source contract.
