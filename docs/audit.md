# Auditoría del primer corte

**Corte:** 2026-09-02

## Decisiones cerradas

- Un binario Rust y un archivo de configuración gobiernan el flujo.
- Windows es la única plataforma del primer corte.
- El host ejecuta un solo cliente mesh nativo.
- El control plane es un prerrequisito, no un caso parcial del binario.
- La dependencia mesh actual queda detrás de `port::mesh`.
- `legacy` es referencia de lectura y no se modifica.

## Gates pendientes

| ID | Evidencia mínima |
|---|---|
| `R-01` | `gnx.exe` compila reproduciblemente y carga `gnx.toml`. |
| `C-01` | Configuración inválida falla antes de mutar el host. |
| `W-01` | `doctor` valida Windows, privilegios y runtime sin mutar. |
| `M-01` | `connect` conserva el endpoint exacto y reporta el estado real. |
| `M-02` | El cliente nativo conserva una sola identidad tras reboot. |
| `M-03` | `join` no puede activar el control plane. |
| `S-01` | Artefactos, versiones, digests, licencias y SBOM están fijados. |
| `S-02` | Git, argv, entorno, logs y evidencia no contienen secretos. |

Fallar cualquiera de estos gates impide `READY`; no se sustituye por una prueba
simulada.

## Riesgos concretos

- Instalación y recuperación del cliente nativo sin sesión abierta.
- Login automatizado sin exponer material sensible; queda deshabilitado hasta
  demostrar un canal seguro.

## No evaluado todavía

Control plane, WSL, contenedores, Linux, Proxmox, routing, proxy, UI, HA y
actualización automática. No generan módulos, flags ni configuración hasta
entrar explícitamente al corte.

## Fuentes primarias

- [NetBird self-hosted](https://docs.netbird.io/selfhosted/selfhosted-quickstart)
- [NetBird CLI](https://docs.netbird.io/get-started/cli)
- [NetBird para Windows](https://docs.netbird.io/get-started/install/windows)
- [Podman Machine](https://docs.podman.io/en/latest/markdown/podman-machine.1.html)
- [Red de WSL](https://learn.microsoft.com/en-us/windows/wsl/networking)
