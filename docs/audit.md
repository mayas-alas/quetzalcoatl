# Auditoría del primer corte

**Corte:** 2026-09-02

## Decisiones cerradas

- Un binario Rust y un archivo de configuración gobiernan el flujo.
- Windows conserva el cliente; WSL aloja el control plane local.
- GNX gestiona un cliente mesh nativo; el operador añadió un cliente de acceso
  Windows para probar el nodo WSL, sin sustituir la identidad gestionada.
- El control plane es un prerrequisito, no un caso parcial del binario.
- La dependencia mesh actual queda detrás de `port::mesh`.
- El primer servicio se prepara con `ops/compute` y un Quadlet independiente.
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
cerrar todos los gates de producto: custodia externa de la clave, restore y
otro host siguen pendientes. Un fallo nunca se sustituye por una prueba simulada.

## Evidencia local

| Comprobación | Resultado 2026-09-02 |
|---|---|
| Tests Rust | `PASS` — 21 del CLI/cliente/credenciales + 3 del bootstrap + 2 del cifrado + 2 de cómputo + 9 de acceso |
| Clippy con warnings como error | `PASS` |
| RustSec sobre los cuatro lockfiles | `PASS` — sin vulnerabilidades conocidas en dependencias Rust |
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
| Reboot Windows | `PASS` — arranque 2026-09-02 10:46:36 UTC−06; servicios, HTTPS y conexión recuperados; un peer con el ID original protegido |
| Cifrado y detección de corrupción | `PASS` unitario — roundtrip; rechazo de clave incorrecta, truncado y modificación |
| Respaldo físico | `PASS` — captura consistente cifrada a las 17:46:32 UTC; descifrado completo y SHA-256 verificados |
| Copia USB | `PASS` — 33 731 662 bytes en `D:/GNX-backups`; SHA-256 idéntico y descifrado completo desde USB; sin clave en la unidad |
| Salud tras respaldo | `PASS` — tres servicios activos, cliente conectado y HTTPS 200 con validación TLS |
| Servicio de cómputo | `PASS` — Quadlet activo, imagen por digest, KVM API 12; `pve-manager` 9.2.11 |
| HTTPS de cómputo | `PASS` — `proxmox.mesh.gnx`, HTTP 200, TLS válido y upstream con CA verificada; sin puertos publicados por cómputo |
| Login y reinicio de cómputo | `PASS` — API autenticada antes y después de reiniciar el servicio; ejecución elevada con código 0 |
| Reboot completo con cómputo | `PENDIENTE` — no lo cubre el reboot previo del control |
| Respaldo de cómputo | `PENDIENTE` — la copia USB verificada sólo cubre el control plane |
| Operador de acceso | `PASS` — build, validación y nodo Quadlet activo sin alterar los HTTPS existentes |
| DNS de acceso aislado | `PASS` — UDP/TCP, wildcard, AAAA y límites de resolución; puertos loopback temporales retirados |
| Claves de acceso | `PASS` — entrada de consola sin eco; rechazo de argv/redirección; archivo tmpfs `0600` eliminado tras éxito/fallo simulado |
| Formulario DNS en CLI | `PASS` — dominio/switches correctos; nameserver pendiente sin conexión; no publica una IP ficticia |
| Enrolamiento de acceso | `PASS` — humano enroló el nodo; IP e identidad guardadas, sin reenrolamiento |
| Política y DNS SaaS | `PASS` — regla mínima recibida; Split DNS privado y consultas UDP/TCP verificadas |
| MTU y HTTPS remoto | `PASS` — fallo reproducido con uplink 1280, tres descargas completas de ambos HTTPS con 1500; servicio de arranque habilitado y ordenado antes de la VPN |
| Android | Operador confirmó ambos dominios; peer observado en Wi-Fi. Datos móviles y confianza TLS completa pendientes |
| Consulta de credenciales | `PASS` — fixture DPAPI y pantalla alternativa en consola, revelado/ocultado con Enter; rechazo de redirección; cuentas reales presentes sin imprimir contraseñas |
| Reboot / respaldo de acceso | `PENDIENTE` — no los cubre el reboot ni la copia USB anteriores |

El bundle contiene un MSI 0.77.1 cuyo digest y firma Authenticode se validaron,
y el cliente quedó instalado. Los intentos previos fallaron con código MSI 2:
primero por el prefijo de ruta extendida y después por separadores mezclados.
Se corrigieron ambos casos y la captura de versión; el reintento físico pasó.
El diagnóstico del MSI permanece fuera de Git en
`%TEMP%/gnx-mesh-client-install.log` y no recibe credenciales de enrolamiento.
Se ejecutó `connect` contra `mesh.gnx`, también después de reiniciar el cliente.
Tras reboot completo se recuperaron servicios, HTTPS y conexión con la misma
IP. La comprobación elevada confirmó un solo peer con el ID original protegido;
el gate `M-02` pasó en este host. El primer UAC de respaldo se canceló antes de
ejecutar; el reintento autorizado terminó con código 0 y copia USB verificada.

La primera entrada HTTPS falló por una directiva no soportada por Podman 4.9.3;
se sustituyó por su argumento compatible. Windows también detectó ausencia de
CRL: se añadió publicación y renovación, sin omitir la comprobación de
revocación. Ambos reintentos pasaron. [Operación y límites](control.md).

En acceso se corrigieron el manejo de IPs nulas antes del login y la aplicación
de ACL sin `SeSecurityPrivilege`. Las pruebas iniciales también detectaron una
aserción de plantillas demasiado amplia y el uso de `.invalid`, que no prueba
forwarding público; se corrigieron y repitieron. El humano completó luego el
enrolamiento. [Estado de acceso](access.md).
El ingreso manual mediante archivo se sustituyó por `gnx access configure`:
la clave queda bajo control humano y su custodia temporal es automática. No
se usaron credenciales reales para comprobar el prompt ni la limpieza.

La sonda DNS se movió a la red del host para evitar el hairpin del contenedor.
La política vacía se reportó y el operador la sustituyó por una regla mínima.
Después se reprodujeron pérdidas de paquetes grandes por MTU WSL 1280;
uplink 1500 resolvió las descargas remotas sin cambiar MTU del túnel, DNS ni CA.
El primer apply tras reiniciar el nodo leyó un estado transitorio: ahora espera
un estado utilizable. El helper DPAPI fija módulos de Windows PowerShell para
no heredar módulos incompatibles de PowerShell 7, fallo detectado con fixture.

## Riesgos concretos

- Instalación y recuperación del cliente nativo sin sesión abierta.
- Custodia de la clave fuera del host y restauración operativa aún pendientes.
- Cómputo privilegiado y control plane comparten WSL y red de contenedores.
- Falta respaldo del estado de cómputo y rotación coordinada de su credencial.
- La identidad de acceso no está cubierta por el respaldo USB; confianza TLS
  Android, datos móviles y renovación de identidad siguen pendientes.
- El login automatizado cubre bootstrap del control y API de cómputo locales.
  No se declara una solución genérica de custodia de secretos.

## No evaluado todavía

Cliente Linux, routing, publicación genérica de aplicaciones, HA,
VMs/LXC, consola WebSocket, restore, reboot con cómputo y actualización
automática. La interfaz web no tiene un recorrido interactivo completo
verificado. [Alcance de cómputo](compute.md). La revisión de
dependencias Rust no sustituye un escaneo de vulnerabilidades de las imágenes.

## Fuentes primarias

- [NetBird self-hosted](https://docs.netbird.io/selfhosted/selfhosted-quickstart)
- [NetBird CLI](https://docs.netbird.io/get-started/cli)
- [NetBird para Windows](https://docs.netbird.io/get-started/install/windows)
- [Podman Machine](https://docs.podman.io/en/latest/markdown/podman-machine.1.html)
- [Red de WSL](https://learn.microsoft.com/en-us/windows/wsl/networking)
