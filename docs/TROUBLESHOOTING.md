# Troubleshooting

## Topology

The control plane runs as `NT SERVICE\Quetzalcoatl`. It owns the Podman Machine
`quetzalcoatl` on the WSL2 provider. Fedora/KVM runs inside that WSL distro.

The interactive user (`nitro\mayas`) cannot enter the Podman machine directly.
All machine operations go through the service account via:

```powershell
podman machine ssh --username root quetzalcoatl
```

Interactive `podman machine inspect quetzalcoatl` returns incomplete or stale data
because the active WSL distro belongs to the service account. Inspect from the
service context only.

## MACHINE_CREATE_FAILED (exit 125)

Two distinct causes produce the same exit code:

### Primary: WSL distro not running

When the WSL distro owned by the service account is down, Podman Machine cannot
complete the SSH handshake. The service reports `MACHINE_CREATE_FAILED` with the
message "machine is not listening on ssh port". This is the primary failure mode
identified during root-cause analysis of the 0xe0434352 crash path in
`Quetzalcoatl.Service.exe`.

### Secondary: Docker-API pipe contention

A Docker-compatible consumer (Docker Desktop, Rancher Desktop, or similar) holds
`\\.\pipe\docker_engine` exclusively. Podman Machine creation or startup fails at
the pipe-acquisition step. The service now detects this condition and reports
`MACHINE_PIPE_CONTENTION` before returning exit 125.

## Recovery

### 1. Check service health

```powershell
sc qc Quetzalcoatl

Get-EventLog -LogName Application -Source Quetzalcoatl*
```

Look for the 0xe0434352 crash signature or any `Quetzalcoatl.Service` error.

### 2. Restart the service

```powershell
Restart-Service Quetzalcoatl
```

This re-establishes the service-owned WSL distro and re-acquires the SSH port.

### 3. Verify SSH port

```powershell
Get-NetTCPConnection -LocalPort 58677
```

Port 58677 must be bound by the `Quetzalcoatl` service context. Absence confirms
the machine has not started.

### 4. Verify machine state (service context)

Run from an elevated session that can access the service account context:

```powershell
podman machine list --format json
podman machine inspect quetzalcoatl
```

Interactive contexts may return stale data. If the machine is corrupted, proceed
to full reset.

### 5. Full reset (corrupted state)

```powershell
Stop-Service Quetzalcoatl

# Remove stale lock files held by the service account
# The exact lock location is determined by the service account profile;
# removing it allows the service to recreate the machine cleanly.

Restart-Service Quetzalcoatl
```

Let the service recreate the machine from its persisted host profile. Do not
delete `runtime/` or `installer-inputs.bin`; the host profile and DPAPI secrets
are independent of machine state.

### 6. Pipe contention

If `MACHINE_PIPE_CONTENTION` is reported, or `podman machine start` fails with a
pipe error:

```powershell
# Stop Docker-compatible consumers
# (Docker Desktop, Rancher Desktop, colima, etc.)

wsl --shutdown

# Retry after all Docker-API consumers release the pipe
podman machine start quetzalcoatl
```

After releasing the pipe, restart the Quetzalcoatl service.

## Diagnostic commands

```powershell
# Machine list (JSON, service context)
podman machine list --format json

# Single machine details (service context)
podman machine inspect quetzalcoatl

# WSL distro state
wsl --list --verbose

# SSH port listener on Windows host
Get-NetTCPConnection -LocalPort 58677

# Service configuration
sc qc Quetzalcoatl

# Application log events from the service
Get-EventLog -LogName Application -Source Quetzalcoatl*
```

## Escalation

If the machine recreates but `podman machine ssh --username root quetzalcoatl`
still fails after the steps above, collect the output of all diagnostic commands
and the `Application` event log before changing WSL or Podman installation state.
The service reconciler is the sole owner of the machine image; manual WSL or
Podman changes must not bypass Setup's validated recovery path.
