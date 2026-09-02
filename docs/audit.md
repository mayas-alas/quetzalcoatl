# Auditoría del primer corte

**Corte:** 2026-09-02

## Decisiones cerradas

- Un binario Rust y un archivo de configuración gobiernan el flujo.
- Windows conserva el cliente; WSL aloja el control plane local.
- El host ejecuta un solo cliente mesh nativo.
- El control plane es un prerrequisito, no un caso parcial del binario.
- La dependencia mesh actual queda detrás de `port::mesh`.
- `legacy` es referencia de lectura y no se modifica.

## Gates de producto

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

`READY` sólo describe la operación cuyos checks se ejecutaron. No equivale a
cerrar todos los gates de producto: comparar el ID tras reboot, backup físico,
restore y otro host siguen pendientes. Un fallo nunca se sustituye por una
prueba simulada.

## Evidencia local

| Comprobación | Resultado 2026-09-02 |
|---|---|
| Tests Rust | `PASS` — 13 del cliente + 3 del bootstrap + 2 del cifrado |
| Clippy con warnings como error | `PASS` |
| RustSec sobre ambos lockfiles | `PASS` — sin vulnerabilidades conocidas en dependencias Rust |
| Build release y checksum del EXE | `PASS` |
| `gnx doctor` físico | `PASS` — cliente 0.77.1, sin elevación |
| Instalación elevada | `PASS` — MSI y GNX devolvieron 0 |
| Servicio local | `PASS` — activo y con arranque automático |
| Instalación repetida | `PASS` — no reinstala ni requiere elevación |
| Control plane WSL | `PASS` — tres servicios activos e imágenes por digest |
| DNS local y TLS Windows | `PASS` — `mesh.gnx`, HTTP 200, cadena/nombre/revocación válidos |
| Enrolamiento | `PASS` — cuenta local, clave one-off y un peer conectado |
| Gestión, señal y transporte | `PASS` — gestión y señal conectadas; STUN y relay disponibles |
| Reinicio de cliente | `PASS` — mismo peer tras reiniciar el servicio |
| Credenciales de bootstrap | `PASS` — PAT y clave eliminados en servidor y archivo; propietario protegido por DPAPI |
| Rutinas del host | `PASS` — tarea de sesión y temporizador de identidad registrados |
| Reboot Windows | `PASS` parcial — arranque 2026-09-02 10:46:36 UTC−06; servicios, HTTPS y misma IP recuperados; ID pendiente |
| Cifrado y detección de corrupción | `PASS` unitario — roundtrip; rechazo de clave incorrecta, truncado y modificación |
| Respaldo físico | `PENDIENTE` — UAC cancelado antes de ejecutar; USB no conectada |

El bundle contiene un MSI 0.77.1 cuyo digest y firma Authenticode se validaron,
y el cliente quedó instalado. Los intentos previos fallaron con código MSI 2:
primero por el prefijo de ruta extendida y después por separadores mezclados.
Se corrigieron ambos casos y la captura de versión; el reintento físico pasó.
El diagnóstico del MSI permanece fuera de Git en
`%TEMP%/gnx-mesh-client-install.log` y no recibe credenciales de enrolamiento.
Se ejecutó `connect` contra `mesh.gnx`, también después de reiniciar el cliente.
Tras reboot completo se recuperaron servicios, HTTPS y conexión con la misma
IP. La comparación con el ID original protegido sigue pendiente de elevación;
el gate `M-02` todavía no se declara cerrado.

La primera entrada HTTPS falló por una directiva no soportada por Podman 4.9.3;
se sustituyó por su argumento compatible. Windows también detectó ausencia de
CRL: se añadió publicación y renovación, sin omitir la comprobación de
revocación. Ambos reintentos pasaron. [Operación y límites](control.md).

## Riesgos concretos

- Instalación y recuperación del cliente nativo sin sesión abierta.
- Backup cifrado implementado, pendiente de ejecución física y copia USB.
- Custodia de la clave fuera del host y restauración operativa aún pendientes.
- El login automatizado validado sólo cubre el bootstrap local por TLS y archivo
  protegido. No se declara una solución genérica de custodia de secretos.

## No evaluado todavía

Cliente Linux, otro host, Proxmox, routing, publicación de aplicaciones, HA,
identidad exacta tras reboot, restore y actualización automática. La consola
HTTP se prueba por respuesta, no mediante un recorrido interactivo completo. La revisión de
dependencias Rust no sustituye un escaneo de vulnerabilidades de las imágenes.

## Fuentes primarias

- [NetBird self-hosted](https://docs.netbird.io/selfhosted/selfhosted-quickstart)
- [NetBird CLI](https://docs.netbird.io/get-started/cli)
- [NetBird para Windows](https://docs.netbird.io/get-started/install/windows)
- [Podman Machine](https://docs.podman.io/en/latest/markdown/podman-machine.1.html)
- [Red de WSL](https://learn.microsoft.com/en-us/windows/wsl/networking)
