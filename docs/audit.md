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
| `R-01` | `gnx.exe` compila con lockfile y carga `gnx.toml`. |
| `C-01` | Configuración inválida falla antes de mutar el host. |
| `W-01` | `doctor` valida Windows, privilegios y runtime sin mutar. |
| `M-01` | `connect` conserva el endpoint exacto y reporta el estado real. |
| `M-02` | El cliente nativo conserva una sola identidad tras reboot. |
| `M-03` | `join` no puede activar el control plane. |
| `S-01` | Artefactos, versiones, digests y licencias están fijados. |
| `S-02` | Git, argv, entorno, logs y evidencia no contienen secretos. |

Fallar cualquiera de estos gates impide `READY`; no se sustituye por una prueba
simulada.

## Evidencia local

| Comprobación | Resultado 2026-09-02 |
|---|---|
| Tests Rust | `PASS` — 13 pruebas, incluida regresión de rutas MSI |
| Clippy con warnings como error | `PASS` |
| RustSec sobre `Cargo.lock` | `PASS` — sin vulnerabilidades conocidas |
| Build release y checksum del EXE | `PASS` |
| `gnx doctor` físico | `PASS` — cliente 0.77.1, sin elevación |
| Instalación elevada | `PASS` — MSI y GNX devolvieron 0 |
| Servicio local | `PASS` — activo y con arranque automático |
| Instalación repetida | `PASS` — no reinstala ni requiere elevación |

El bundle contiene un MSI 0.77.1 cuyo digest y firma Authenticode se validaron,
y el cliente quedó instalado. Los intentos previos fallaron con código MSI 2:
primero por el prefijo de ruta extendida y después por separadores mezclados.
Se corrigieron ambos casos y la captura de versión; el reintento físico pasó.
El diagnóstico del MSI permanece fuera de Git en
`%TEMP%/gnx-mesh-client-install.log` y no recibe credenciales de enrolamiento.
No se ejecutó `connect`; enrolamiento, conectividad y persistencia tras reboot
siguen sin validar.

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
