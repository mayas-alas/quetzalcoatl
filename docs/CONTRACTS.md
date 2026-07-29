# Product contracts

## IPC and state

CLI and tray use only the existing Named Pipe commands `status`, `configure` and
`restart` with protocol schema 2. `gnx version` is local. Serialized status values
use closed types and invalid or unknown values fail closed.

Persisted runtime state remains schema 2; the host profile remains schema 1.
Runtime state preserves identity, role, controller, tailnet and the bounded
member-join checkpoint. Writes are validated and atomic. Installer recovery is a
separate schema-2 journal and retains the 0.1.17 migration path.

One closed bootstrap preflight selects CPU, RAM and disk. The same persisted profile
drives both service `.wslconfig` and Podman Machine creation; callers cannot inject
arbitrary resource values.

## Runtime payload

`runtime/manifest.toml` fixes generation `proxmox-cluster-v2` and payload contract 5.
`runtime/payload.lock.json` is authoritative for components, installed paths, modes
and SHA-256 values.

```text
runtime/
|-- commands/        installed executable payload only
|-- configuration/   installed configuration
|-- containers/      installed Quadlet definitions
|-- services/        installed units
|-- operations/      repository-owned stdin programs and probes
|-- manifest.toml
`-- payload.lock.json
```

Only locked files are installed. Operations are compiled into fixed orchestration
paths and are not copied as runtime payload.

## Remote execution

Remote execution has three distinct channels:

```text
argv  = closed repository-selected operation
stdin = bounded variable data or repository-owned program
file  = validated durable state required by a consumer
```

Argument vectors contain no shell syntax, redirection, pipelines, substitution,
caller commands or arbitrary remote argv. Multiline programs use a fixed
interpreter in stdin mode (`sh -s` or `python3 -`). Input, output and time are
bounded; timeout kills and reaps the child. Durable writes use GNX-owned paths,
restrictive permissions and atomic activation. Secret-bearing stdin is never logged.

`sh -c`, `bash -c`, new listeners and mutable image tags are prohibited. The
exception set is empty.
