# Seguimiento del PoC Quetzalcoatl

Última actualización: 2026-07-18  
Estado global: `VALIDACIÓN I1 EN CURSO`

Siguiente trabajo: `A-03 · ejecutar el EXE elevado y verificar servicio, CLI y SID estable`

## 1. Objetivo de seguimiento

Este archivo responde solamente cuatro preguntas:

1. ¿Cuál es el siguiente trabajo que acerca al instalador funcional?
2. ¿Qué está realmente bloqueado?
3. ¿Qué evidencia demuestra que algo funciona en un host real?
4. ¿Se está agregando código fuera de los dos incrementos?

No es un backlog de producto futuro. No se crearán documentos adicionales de planificación.

## 2. Reglas de ejecución

- Sólo existen el Gate 0, el Incremento 1 y el Incremento 2.
- Hay como máximo un trabajo `EN CURSO`.
- Ninguna idea futura entra a esta tabla.
- Una tarea se cierra con evidencia de ejecución, no porque compile.
- No se crea un framework de pruebas; se conservan comandos, salidas y hashes de la aceptación manual.
- Si aparece un stopper, se resuelve el mismo camino técnico. No se implementa un proveedor o fallback alternativo.
- WiX 5, rol automático, `auth_key` y Docker dentro de LXC son decisiones cerradas.
- `runtime payload v1` son archivos, hashes, Quadlets y scripts fijados; no es un subsistema ni un framework de migraciones.
- Quadlet administra el runtime Podman local. OpenTofu se ejecuta bajo demanda y no es un daemon.
- Un cambio de alcance requiere una instrucción explícita del usuario y una modificación previa de [ARCHITECTURE.md](./ARCHITECTURE.md).

Estados de trabajo permitidos: `NO INICIADO`, `EN CURSO`, `BLOQUEADO`, `CERRADO`. Los stoppers usan únicamente `ABIERTO` o `CERRADO`.

## 3. Estado comprobado actual

- La definición original está en `PoC.md`.
- La arquitectura normativa está en `docs/ARCHITECTURE.md`.
- El workspace Rust contiene HostPreflight, servicio, CLI y contrato Named Pipe local.
- Burn/MSI incorporan WinSW, WSL 2.7.10, Podman 6.0.1 y el `runtime payload v1` fijado.
- Existe un `QuetzalcoatlSetup.exe` estáticamente validado; todavía no ha completado una instalación elevada.
- El payload de PVE, Tailscale, OpenTofu, Garage y Forgejo está fijado por manifest; su ejecución real pertenece a A-04/A-05.
- HostPreflight pasó la ruta elevada en este Windows después de reiniciar; el host aún conserva Podman 6.0.0 hasta ejecutar el bundle.

## 4. Resultado de los dos incrementos

| ID | Resultado observable | Estado | Evidencia de cierre | Bloqueos de cierre |
|---|---|---|---|---|
| I1 | En Windows limpio, el EXE instala o reanuda WSL2, valida KVM, instala Podman, crea la máquina administrada, registra Tailscale, detecta cero hosts GNX, queda controller, levanta PVE y ejecuta OpenTofu. La aceptación canónica selecciona Garage y Forgejo; ambos quedan operativos. `gnx status --json` termina `READY`. | `EN CURSO` | Hash del EXE; API KVM; inventario estable que excluye self/sidecars; `Self.ID` y rol persistidos; `pvecm status`; state OpenTofu; S3 PUT/GET; push/clone Forgejo; bootstrap PVE reemplazado; ausencia de secretos persistidos; `gnx status --json` | G0-01, G0-02, G0-05, G0-07 y B-02 cerrados |
| I2 | En un segundo Windows, el mismo EXE encuentra exactamente el controller autorizado, queda member, levanta PVE, ejecuta `pvecm join`, no ejecuta OpenTofu y no recrea singletons. | `NO INICIADO` | `gnx status --json`; `pvecm nodes/status` en ambos nodos; Tailscale directo; SSH/Corosync; rol/controller ID persistidos; intento OpenTofu denegado antes de ejecutar; member sin workspace/state/credenciales; una sola instancia de cada servicio remoto | I1, G0-03, G0-04 y G0-06 cerrados |

I1 no puede cerrarse mientras B-07 siga abierto. I2 no puede cerrarse mientras B-06 siga abierto.

## 5. Gate 0 — factibilidad por incremento

Gate 0 no es un tercer incremento. Cada gate debe cerrarse antes del camino de código que depende de él; no es necesario esperar los gates exclusivos de I2 para producir y cerrar I1.

| ID | Resultado requerido | Estado | Evidencia |
|---|---|---|---|
| G0-01 | WSL2 y Podman Machine exponen KVM utilizable | `NO INICIADO` | `KVM_GET_API_VERSION=12` dentro de la máquina |
| G0-02 | El contenedor PVE privilegiado arranca con KVM, TUN, FUSE, cgroup v2 y persistencia | `NO INICIADO` | `pvesh get /version` y `pvecm status` antes/después de reinicio; `/etc/pve` preservado |
| G0-03 | Dos nodos Tailscale con tag de producto obtienen camino directo y RTT menor a 5 ms | `NO INICIADO` | `tailscale ping`, pérdida y RTT |
| G0-04 | PVE API/SSH/Corosync funcionan por la tailnet sin puertos Windows | `NO INICIADO` | Relojes sincronizados; probes TCP 22/8006; tráfico UDP 5405-5412 capturado sobre tailnet; ACL/firewall efectivos; cero listeners PVE en Windows |
| G0-05 | Los LXC PVE ejecutan los Compose canónicos de Garage y Forgejo con TUN y `fuse-overlayfs` después de reiniciar | `NO INICIADO` | `docker info`; ambos sidecars saludables; S3 PUT/GET y push/clone Forgejo después de reinicio |
| G0-06 | El segundo PVE se une de forma no interactiva y controlada al primero | `NO INICIADO` | `pvecm nodes/status` con dos nodos y quorum; join reanudable; password ausente de argv, archivos y logs |
| G0-07 | Tailscale Serve HTTPS funciona sin consentimiento interactivo | `NO INICIADO` | `CertDomains` esperado; PVE, S3 y Forgejo accesibles por HTTPS; `AllowFunnel=false` |

El registro Gate 0 completo queda `CERRADO` cuando G0-01 a G0-07 tienen evidencia. La columna “Bloqueos de cierre” indica el subconjunto que cada incremento debe resolver, incluso si se cierra durante su integración vertical.

## 6. Stopper register

Sólo se registran brechas de factibilidad o seguridad que bloquean I1 o I2. No se agregan posibilidades futuras; cada stopper se cierra con evidencia del mismo camino técnico.

| ID | Stopper | Impacto | Condición de cierre | Estado |
|---|---|---|---|---|
| B-01 | WSL2 → Podman Machine → KVM aún no está demostrado | Impide PVE | Gate obtiene `KVM_GET_API_VERSION=12` desde la máquina y el contenedor privilegiado | `ABIERTO` |
| B-02 | Imagen PVE, Tailscale, OpenTofu, Quadlets y Compose no están fijados por digest/commit | Runtime no reproducible | Manifest v1 contiene fuente, versión, digest y hash de cada entrada | `CERRADO` |
| B-03 | Arranque y persistencia de PVE OCI privilegiado no demostrados | Impide controller y member | PVE vuelve saludable después de reiniciar máquina/contenedor sin perder estado | `ABIERTO` |
| B-04 | Docker dentro de LXC con TUN/FUSE/cgroup no demostrado | Impide Garage y Forgejo | Los Compose canónicos sobreviven reinicio y ambos sidecars quedan saludables | `ABIERTO` |
| B-05 | No existe evidencia de camino tailnet directo dentro del límite de Corosync | Impide clúster estable | Ambos hosts muestran camino directo, pérdida cero y RTT menor a 5 ms | `ABIERTO` |
| B-06 | Canal no interactivo de `pvecm join` y credencial protegida no demostrado | Impide I2 | Join repetible, sin password en argv/logs/archivos planos | `ABIERTO` |
| B-07 | Handoff Burn → servicio → DPAPI → Linux no demostrado | Impide cerrar I1 con manejo seguro de secretos | Integración sin secreto en log, MSI property, argv, Compose, contenedor permanente ni state; `/run` eliminado | `ABIERTO` |
| B-08 | HTTPS de Tailscale Serve no está demostrado como prehabilitado | Impide UI PVE y endpoints de Garage/Forgejo desatendidos | `CertDomains` válido y los tres endpoints funcionan sin URL de consentimiento | `ABIERTO` |

Un hallazgo que no bloquee alguno de los dos incrementos no pertenece aquí.

## 7. Plan de implementación inmediato

| Orden | ID | Trabajo | Estado | Terminado cuando |
|---:|---|---|---|---|
| 1 | A-01 | Crear el workspace Rust único e implementar `HostPreflight` Windows/WSL2 | `CERRADO` | Un binario reusable entrega códigos estables; no captura secretos |
| 2 | A-02 | Fijar referencias externas y construir `runtime manifest v1` | `CERRADO` | Cierra B-02 sin copiar contenido no utilizado |
| 3 | A-03 | Crear WiX 5 Burn/MSI + WinSW, identidad runtime y primer EXE | `EN CURSO` | Setup reanuda reboot, instala servicio/CLI y mantiene el mismo SID |
| 4 | A-04 | Implementar `RuntimeGate` dentro de `gnx-service` | `NO INICIADO` | La identidad dedicada crea la máquina y cierra G0-01 y B-01 |
| 5 | A-05 | Integrar verticalmente I1, sin desarrollar I2 en paralelo | `NO INICIADO` | Cierra G0-02, G0-05, G0-07, B-03, B-04, B-07, B-08 y toda evidencia I1 |
| 6 | A-06 | Probar red directa de dos hosts, `pvecm create/add` y canal protegido de join | `NO INICIADO` | Cierra G0-03, G0-04, G0-06, B-05 y B-06 |
| 7 | A-07 | Implementar únicamente descubrimiento y join de I2 | `NO INICIADO` | Toda la evidencia I2 está registrada |

La siguiente acción siempre es la primera fila no cerrada. No se inicia una fila posterior “para avanzar en paralelo” si la anterior define su contrato.

## 8. Desglose de Incremento 1

| ID | Entregable | Estado | Dependencia |
|---|---|---|---|
| I1-01 | Burn HostPreflight, checkpoint de reboot y MSI base | `EN CURSO` | A-03 |
| I1-02 | Cuenta dedicada, WinSW, `gnx-service` y Named Pipe | `NO INICIADO` | I1-01 |
| I1-03 | RuntimeGate, máquina `quetzalcoatl` y aplicación de payload v1 | `NO INICIADO` | I1-02, A-04 |
| I1-04 | Quadlets de Tailscale/PVE y OpenTofu one-shot | `NO INICIADO` | I1-03 |
| I1-05 | DPAPI y `gnx-tailscale-enroll` one-shot sólo con `auth_key` | `NO INICIADO` | I1-02 |
| I1-06 | Descubrimiento cero peers y persistencia controller | `NO INICIADO` | I1-04, I1-05 |
| I1-07 | `pvecm create` y PVE privado saludable | `NO INICIADO` | I1-06 |
| I1-08 | OpenTofu local state y LXC seleccionados | `NO INICIADO` | I1-07 |
| I1-09 | Garage/Forgejo mediante Docker Compose y secretos DPAPI | `NO INICIADO` | I1-08 |
| I1-10 | `gnx status --json`, EXE y aceptación real | `NO INICIADO` | I1-01 a I1-09 |

## 9. Desglose de Incremento 2

I2 no comienza hasta que I1 está cerrado.

| ID | Entregable | Estado | Dependencia |
|---|---|---|---|
| I2-01 | Descubrimiento de exactamente un peer host controller | `NO INICIADO` | I1 cerrado, G0-03 |
| I2-02 | Persistencia member y controller ID/IP | `NO INICIADO` | I2-01 |
| I2-03 | PVE member limpio y preflight de red cluster | `NO INICIADO` | I2-02, G0-04 |
| I2-04 | `pvecm join` protegido | `NO INICIADO` | I2-03, G0-06 |
| I2-05 | Bloqueo verificable de OpenTofu y servicios singleton | `NO INICIADO` | I2-02 |
| I2-06 | Estado del clúster, mismo EXE y aceptación real | `NO INICIADO` | I2-01 a I2-05 |

## 10. Evidencia

| Fecha | ID | Host | Artefacto o comando | Resultado | Ruta/hash |
|---|---|---|---|---|---|
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `cargo fmt --all -- --check` + `cargo check --workspace` | Formato y compilación correctos | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 - desarrollo sin admin | `cargo build --release -p gnx-host-preflight` | EXE release generado; JSON fail-stop reproduce exit 11 | SHA-256 `FAB9A0CBA8769A2C413592ADE9E5A733B3FB015B856482FEDF400A9552E0EB56` - commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `gnx-host-preflight --format json` | `windows_host` pass, elevación fail, salida JSON única y exit 11 | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `gnx-host-preflight --format yaml` | Uso rechazado por stderr y exit 64 | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · ejecución elevada | `gnx-host-preflight --format json` | Detectó y corrigió falsos negativos en hipervisor y salida OEM de DISM; Windows, elevación, virtualización, WSL y VMP pasan; fail-stop exit 14 por reinicio pendiente real | SHA-256 `154ADAF4928D3731FF8757DE90F4E4408C734AC0CFE361CC518C72545CBA81B7` · commit `acccf66` |
| 2026-07-18 | A-01 | Windows 11 x64 · ejecución elevada después de reinicio | `gnx-host-preflight --format json` | Seis gates previos pasan; la ruta completa alcanza `podman_msi` y rechaza Podman 6.0.0 con exit 16 frente al pin 6.0.1 | SHA-256 `154ADAF4928D3731FF8757DE90F4E4408C734AC0CFE361CC518C72545CBA81B7` · commit `acccf66` |
| 2026-07-18 | A-02/B-02 | Validación estática del payload `linux/amd64` | Manifest, hashes, parsers JSON/TOML/YAML y `dash -n` | 7 componentes, 12 archivos y 7 referencias OCI fijados; LF; `AllowFunnel=false`; sin tags mutables, auth keys ni secretos embebidos | manifest SHA-256 `6A40DA8CDBB10EAECB7BB13F054543E590186CEE60BBA821D4D620AE358923BC` · commit `68e7a16` |
| 2026-07-18 | A-03/I1-01 | Windows 11 x64 · ejecución elevada | `gnx-host-preflight prepare-wsl --format json` | Windows, elevación, virtualización, WSL y VMP pasan; no requirió cambios ni reinicio | `gnx-host-preflight.exe` SHA-256 `C24A4411ABF79A549A6484A2403C8C7397D238C47852552C6579627CAC5A21EE` · commit `5410abd` |
| 2026-07-18 | A-03 | Windows 11 x64 · servicio en consola | `gnx status --json` contra `\\.\pipe\Quetzalcoatl` | El servidor leyó primero el mensaje, impersonó el token real del cliente y devolvió `SERVICE_READY`; el estado global permaneció `pending` | `gnx-service.exe` SHA-256 `F152E62F7956F1A0BC6E8DD9A4857152F0086DC87FCD5C54541E9F11C909E2E3`; `gnx.exe` SHA-256 `C78F7FC41D33674B115C7712FAED5D688E0A2CB87AD9AD081CE5C55E0D23A0F1` · commit `3caff3b` |
| 2026-07-18 | A-03/I1-01 | Validación estática WiX 5.0.2 | `build.ps1`; `wix msi validate`; decompile MSI; extract Burn | MSI válido con 18 archivos, cuenta `NT SERVICE\Quetzalcoatl`, ProductCode fijo y cadena de 5 paquetes; 3010 programa reinicio; WSL 2.7.10 y Podman 6.0.1 embebidos. El EXE aún está sin firma Authenticode | MSI SHA-256 `02D626D517AC796F6C792A309FC0ACC951323344F3F2D2D8AABE71FBC9417255`; EXE SHA-256 `91A66F2C7E7670869BBEEBE4C5B82BE9BFD27245A3F6B700C332C6F159843645` · commits `7860ade`, `7d0cf23` |

Reglas de evidencia:

- Conservar sólo salida necesaria y redactada.
- No pegar `auth_key`, passwords, tokens o valores DPAPI.
- Para binarios registrar SHA-256.
- Para imágenes registrar digest, no `latest`.
- Para comandos registrar host, versión y resultado observable.
- “Compila” habilita la siguiente ejecución, pero no cierra un incremento.

## 11. Decision log cerrado

| ID | Decisión | Razón de congruencia |
|---|---|---|
| D-01 | Rol automático por `tailscale status --json` | Es el comportamiento solicitado; no existe invitación |
| D-02 | Cero peers host = controller; exactamente uno = member | Cierra la topología de aceptación a dos nodos y evita elección |
| D-03 | Una `auth_key` sin tags propios; `tagOwners` y `--advertise-tags` explícito | Permite identidades host/service distintas sin OAuth ni dos keys |
| D-04 | Tags separados para host y sidecars | Garage/Forgejo no alteran el conteo de rol |
| D-05 | Tailscale, PVE y OpenTofu son obligatorios | Forman el núcleo funcional |
| D-06 | Garage y Forgejo son opcionales y controller-only | Mantiene los dos flags originales sin recreación desde members |
| D-07 | Docker corre dentro de LXC | Restricción explícita, validada mediante Gate 0 |
| D-08 | WiX Toolset 5.0.2 | Restricción de licencia aceptada; se fija versión exacta |
| D-09 | `runtime payload v1` es contenido, no plataforma | Une MSI y Fedora sin crear otro subsistema |
| D-10 | Quadlets sólo administran Podman local | Evita duplicar ownership con Burn, servicio u OpenTofu |
| D-11 | OpenTofu usa state local controller-only | Garage opcional no puede ser backend obligatorio |
| D-12 | Serve publica UI TCP/HTTPS; PVE 8006, SSH y Corosync usan tailnet directa | Mantiene el transporte compatible con cada protocolo |
| D-13 | No hay fallback cuando un gate falla | Evita crecimiento por casos alternos |
| D-14 | Los snippets Tailscale se adaptan a DPAPI y bootstrap transitorio | Ningún `auth_key` queda en Quadlet, Compose o contenedor permanente |
| D-15 | `HostPreflight` pertenece a Burn y `RuntimeGate` al servicio | Respeta el perfil/SID que posee Podman y DPAPI |
| D-16 | Corosync usa `link0` fijado a IP tailnet | Evita que PVE elija otra interfaz |
| D-17 | El bootstrap LXC usa `pct push/exec` antes del sidecar | Elimina el ciclo de depender de SSH/Tailscale aún inexistente |
| D-18 | Podman CLI 6.0.1 queda fijado; HostPreflight identifica producto instalado y Burn valida el paquete por hash | Separa observación del host de instalación sin duplicar ownership |
| D-19 | `NT SERVICE\Quetzalcoatl` es la única identidad runtime | Cuenta virtual sin contraseña, SID ligado al nombre del servicio y perfil cargado por SCM; evita crear otra gestión de credenciales |
| D-20 | WiX 5.0.2, WSL 2.7.10.0 y WinSW 2.12.0 quedan fijados | Una sola cadena verificable; no se incorporan canales alternos ni resolución dinámica de versiones |

## 12. Registro de avance

| Fecha | Cambio | Efecto |
|---|---|---|
| 2026-07-18 | Se cerró la arquitectura y el alcance de dos incrementos | Listo para iniciar A-01 |
| 2026-07-18 | A-01 implementado y contrato fail-stop parcial ejecutado | Permanece `EN CURSO` hasta evidencia elevada completa |
| 2026-07-18 | A-01 alcanzó el gate de reinicio en ejecución elevada real | Se requiere reiniciar el host y reanudar el mismo binario; no se omite ni limpia la señal de Windows |
| 2026-07-18 | A-01 cerró la ruta elevada completa después del reinicio | El Podman 6.0.0 existente fue rechazado correctamente; A-02 queda como único trabajo en curso |
| 2026-07-18 | A-02 cerró B-02 con `runtime payload v1` verificable | A-03 inicia sobre un único conjunto de digests y archivos; los snippets quedan sólo como referencia de topología |
| 2026-07-18 | A-03 produjo y validó estáticamente el primer setup completo | A-03 permanece abierto hasta ejecutar el bundle elevado, comprobar servicio/CLI y demostrar el mismo SID después de reiniciar |

Al actualizar este archivo:

1. cambiar el estado de la tarea activa;
2. registrar evidencia y cerrar o actualizar sólo los stoppers afectados;
3. marcar la siguiente tarea como `EN CURSO`;
4. añadir una línea al registro;
5. no agregar backlog futuro.
