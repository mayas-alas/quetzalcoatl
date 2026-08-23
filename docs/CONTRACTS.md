# Product contracts

## IPC and state

CLI and tray use protocol-schema-2 Named Pipe operations. `status` reads state,
`configure` stores node enrollment/PVE input, and `configure_platform` stores the
separate service-enrollment input. `gnx restart` uses the Windows service manager
and `gnx version` is local. Serialized commands and status values use closed types;
invalid or unknown values fail closed.

Forgejo administration uses the resource hierarchy `gnx forgejo admin show` and
`gnx forgejo admin reset --confirm`. Both operations impersonate the Named Pipe
client and require an elevated local Administrators token. The service accepts no
caller-selected username or password. Show verifies the persisted credential
against Forgejo before returning it; reset generates 24 random bytes, changes the
password through Forgejo's loopback API, verifies the result and atomically commits
the controller copy. Secret values use bounded stdin and the encrypted pipe only;
they are prohibited in argv, process environment, state, logs and errors.

Persisted runtime state remains schema 2; the host profile remains schema 1.
Runtime state preserves identity, role, controller, tailnet and the bounded
member-join checkpoint. Writes are validated and atomic. Installer recovery is a
separate schema-2 journal and retains the 0.1.17 migration path.

One closed bootstrap preflight selects CPU, RAM and disk. The same persisted profile
drives both service `.wslconfig` and Podman Machine creation; callers cannot inject
arbitrary resource values.

## Protected configuration

`gnx configure` and `gnx configure platform` are distinct elevated operations.
Node input remains in `installer-inputs.bin`. Platform input schema 1 contains only
the Tailscale service-enrollment auth key and is stored in
`platform-inputs.bin`. Each blob has distinct DPAPI entropy and is protected for
SYSTEM plus the Quetzalcoatl service SID.

The platform key must be reusable, preauthorized and restricted by the tailnet to
`tag:quetzalcoatl-service`. It is an enrollment credential, not application
configuration: it may be materialized only through bounded secret stdin into a
root-only transient file and must be removed after the LXC establishes persistent
Tailscale state. It is prohibited in repositories, Forgejo Actions secrets,
Compose, `.env`, OCI images, logs, argv and OpenTofu state.

Service enrollment passes only a `file:` reference from a root-only declarative
`tailscaled` configuration to a transient, digest-pinned container. The
credential itself never appears in CLI arguments or process environment; the
container, configuration and key are removed after the persistent sidecar state
obtains its node identity. Permanent Compose definitions contain neither an auth
key nor its path.

## Runtime payload

`runtime/manifest.toml` fixes generation `proxmox-platform` and payload contract 6.
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
interpreter in stdin mode (`sh -s` or `python3 -`). The platform applies that same
contract through `pct exec <known-vmid> -- /bin/sh -s`; OpenTofu contains no
provisioners. Input, output and time are bounded; timeout kills and reaps the child.
Durable writes use GNX-owned paths, restrictive permissions and atomic activation.
Secret-bearing stdin is never logged.

Platform reconciliation, deployment and Forgejo administration share one
controller-owned exclusive lock. A password reset cannot overlap an operation that
may consume the previous credential.

`sh -c`, `bash -c`, new listeners and mutable image tags are prohibited. The
exception set is empty.
