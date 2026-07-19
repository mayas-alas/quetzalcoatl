# Seguimiento del PoC Quetzalcoatl

Última actualización: 2026-07-19
Estado global: `VALIDACIÓN I1 EN CURSO`

Siguiente trabajo: `A-05 · cerrar Compose de Garage/Forgejo y sus probes vivos`

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
- Burn/MSI incorporan WinSW, WSL 2.7.10, Podman 6.0.1, la imagen WSL de Podman Machine OS 6.0.1 y el `runtime payload v1` fijado.
- El primer bundle completó una instalación elevada y permitió descubrir un defecto real del resolvedor OCI de Podman 6.0.1. El camino quedó corregido con el artefacto oficial embebido, sin fallback ni resolución por red.
- El `QuetzalcoatlSetup.exe` final completó la instalación elevada con exit 0 después de limpiarse la señal real de reboot. Tras el reinicio de Windows del 2026-07-19, el servicio volvió en modo `Auto` con la misma cuenta y SID, y la máquina persistida regresó a `KVM_READY`.
- RuntimeGate verifica y aplica 30 archivos fijados, levanta el Quadlet PVE y alcanza `PROXMOX_READY` sólo después de obtener KVM API 12, TUN y FUSE dentro de la máquina y del contenedor, más systemd, cgroup v2 y `pvesh` saludables.
- El host conserva WSL 2.7.10 y Podman 6.0.1. El bundle 0.1.1 y su MSI hicieron major upgrade transaccional de 0.1.0; el producto, servicio, CLI y payload instalados coinciden con el build release.
- DPAPI, Tailscale, Serve PVE, rol controller, clúster quorate, persistencia Corosync y OpenTofu están demostrados bajo el SID real del servicio. Docker 29.6.2 quedó instalado y verificado dentro de ambos LXC; Garage alcanzó `docker compose up` y espera el diagnóstico final de salud. Forgejo aún no inicia. Dos hosts Dockur Windows están vivos y accesibles para repetir I2, cuyo código no se ha iniciado.

## 4. Resultado de los dos incrementos

| ID | Resultado observable | Estado | Evidencia de cierre | Bloqueos de cierre |
|---|---|---|---|---|
| I1 | En Windows limpio, el EXE instala o reanuda WSL2, valida KVM, instala Podman, crea la máquina administrada, registra Tailscale, detecta cero hosts GNX, queda controller, levanta PVE y ejecuta OpenTofu. La aceptación canónica selecciona Garage y Forgejo; ambos quedan operativos. `gnx status --json` termina `READY`. | `EN CURSO` | Hash del EXE; API KVM; inventario estable que excluye self/sidecars; `Self.ID` y rol persistidos; `pvecm status`; state OpenTofu; S3 PUT/GET; push/clone Forgejo; bootstrap PVE reemplazado; ausencia de secretos persistidos; `gnx status --json` | G0-01, G0-02, G0-05, G0-07 y B-02 cerrados |
| I2 | En un segundo y un tercer Windows, el mismo EXE encuentra exactamente el controller autorizado entre uno o dos peers, queda member, levanta PVE, ejecuta `pvecm join`, no ejecuta OpenTofu y no recrea singletons. | `NO INICIADO` | `gnx status --json` en ambos members; `pvecm nodes/status` con tres nodos; Tailscale directo entre todos; SSH/Corosync; rol/controller ID persistidos; intento OpenTofu denegado antes de ejecutar; members sin workspace/state/credenciales; una sola instancia de cada servicio remoto | I1, G0-03, G0-04 y G0-06 cerrados |

I1 no puede cerrarse mientras B-04 y B-08 sigan abiertos. I2 no puede cerrarse mientras B-06 siga abierto.

## 5. Gate 0 — factibilidad por incremento

Gate 0 no es un tercer incremento. Cada gate debe cerrarse antes del camino de código que depende de él; no es necesario esperar los gates exclusivos de I2 para producir y cerrar I1.

| ID | Resultado requerido | Estado | Evidencia |
|---|---|---|---|
| G0-01 | WSL2 y Podman Machine exponen KVM utilizable | `CERRADO` | `KVM_GET_API_VERSION=12` dentro de la máquina |
| G0-02 | El contenedor PVE privilegiado arranca con KVM, TUN, FUSE, cgroup v2 y persistencia | `CERRADO` | Arranque/probes pasan; clúster permanece joined/quorate y conserva su authkey Corosync tras reinicios del servicio/contenedor |
| G0-03 | Los tres nodos Tailscale con tag de producto obtienen camino directo por pares y RTT menor a 5 ms | `NO INICIADO` | `tailscale ping`, pérdida y RTT de controller↔member y member↔member |
| G0-04 | PVE API/SSH/Corosync funcionan por la tailnet sin puertos Windows | `NO INICIADO` | Relojes sincronizados; probes TCP 22/8006; tráfico UDP 5405-5412 capturado sobre tailnet; ACL/firewall efectivos; cero listeners PVE en Windows |
| G0-05 | Los LXC PVE ejecutan los Compose canónicos de Garage y Forgejo con TUN y `fuse-overlayfs` después de reiniciar | `NO INICIADO` | `docker info`; ambos sidecars saludables; S3 PUT/GET y push/clone Forgejo después de reinicio |
| G0-06 | Los dos PVE member se unen de forma no interactiva y controlada al controller | `NO INICIADO` | `pvecm nodes/status` con tres nodos y quorum; ambos joins reanudables; password ausente de argv, archivos y logs |
| G0-07 | Tailscale Serve HTTPS funciona sin consentimiento interactivo | `NO INICIADO` | `CertDomains` esperado; PVE, S3 y Forgejo accesibles por HTTPS; `AllowFunnel=false` |

El registro Gate 0 completo queda `CERRADO` cuando G0-01 a G0-07 tienen evidencia. La columna “Bloqueos de cierre” indica el subconjunto que cada incremento debe resolver, incluso si se cierra durante su integración vertical.

## 6. Stopper register

Sólo se registran brechas de factibilidad o seguridad que bloquean I1 o I2. No se agregan posibilidades futuras; cada stopper se cierra con evidencia del mismo camino técnico.

| ID | Stopper | Impacto | Condición de cierre | Estado |
|---|---|---|---|---|
| B-01 | WSL2 → Podman Machine → KVM aún no está demostrado | Impide PVE | Gate obtiene `KVM_GET_API_VERSION=12` desde la máquina y el contenedor privilegiado | `CERRADO` |
| B-02 | Imagen de máquina, PVE, Tailscale, OpenTofu, Quadlets y Compose no están fijados por digest/commit | Runtime no reproducible | Manifest v1 contiene fuente, versión, digest y hash de cada entrada | `CERRADO` |
| B-03 | Arranque y persistencia de PVE OCI privilegiado no demostrados | Impide controller y member | PVE vuelve saludable después de reiniciar máquina/contenedor sin perder estado | `CERRADO` |
| B-04 | Docker dentro de LXC con TUN/FUSE/cgroup no demostrado | Impide Garage y Forgejo | Los Compose canónicos sobreviven reinicio y ambos sidecars quedan saludables | `ABIERTO` |
| B-05 | No existe evidencia de camino tailnet directo dentro del límite de Corosync | Impide clúster estable | Los tres hosts muestran camino directo por pares, pérdida cero y RTT menor a 5 ms | `ABIERTO` |
| B-06 | Canal no interactivo de `pvecm join` y credencial protegida no demostrado | Impide I2 | Join repetible, sin password en argv/logs/archivos planos | `ABIERTO` |
| B-07 | Handoff Burn → servicio → DPAPI → Linux no demostrado | Impide cerrar I1 con manejo seguro de secretos | Integración sin secreto en log, MSI property, argv, Compose, contenedor permanente ni state; `/run` eliminado | `CERRADO` |
| B-08 | HTTPS de Tailscale Serve no está demostrado como prehabilitado | Impide UI PVE y endpoints de Garage/Forgejo desatendidos | `CertDomains` válido y los tres endpoints funcionan sin URL de consentimiento | `ABIERTO` |

Un hallazgo que no bloquee alguno de los dos incrementos no pertenece aquí.

## 7. Plan de implementación inmediato

| Orden | ID | Trabajo | Estado | Terminado cuando |
|---:|---|---|---|---|
| 1 | A-01 | Crear el workspace Rust único e implementar `HostPreflight` Windows/WSL2 | `CERRADO` | Un binario reusable entrega códigos estables; no captura secretos |
| 2 | A-02 | Fijar referencias externas y construir `runtime manifest v1` | `CERRADO` | Cierra B-02 sin copiar contenido no utilizado |
| 3 | A-03 | Crear WiX 5 Burn/MSI + WinSW, identidad runtime y primer EXE | `CERRADO` | Setup reanuda reboot, instala servicio/CLI y mantiene el mismo SID |
| 4 | A-04 | Implementar `RuntimeGate` dentro de `gnx-service` | `CERRADO` | La identidad dedicada crea la máquina y cierra G0-01 y B-01 |
| 5 | A-05 | Integrar verticalmente I1, sin desarrollar I2 en paralelo | `EN CURSO` | Cierra G0-02, G0-05, G0-07, B-03, B-04, B-07, B-08 y toda evidencia I1 |
| 6 | A-06 | Probar red directa de tres hosts, `pvecm create/add` y canal protegido de join | `NO INICIADO` | Cierra G0-03, G0-04, G0-06, B-05 y B-06 |
| 7 | A-07 | Implementar un único descubrimiento/join de I2 y repetirlo en dos members | `NO INICIADO` | Toda la evidencia I2 está registrada sin crear I3 |

La siguiente acción siempre es la primera fila no cerrada. No se inicia una fila posterior “para avanzar en paralelo” si la anterior define su contrato.

La evidencia de A-04 proviene de la ejecución bajo `NT SERVICE\Quetzalcoatl`; compilar o ejecutar bajo el usuario interactivo no cierra ningún gate.

## 8. Desglose de Incremento 1

| ID | Entregable | Estado | Dependencia |
|---|---|---|---|
| I1-01 | Burn HostPreflight, checkpoint de reboot y MSI base | `CERRADO` | A-03 |
| I1-02 | Cuenta dedicada, WinSW, `gnx-service` y Named Pipe | `CERRADO` | I1-01 |
| I1-03 | RuntimeGate, máquina `quetzalcoatl` y aplicación de payload v1 | `CERRADO` | I1-02, A-04 |
| I1-04 | Quadlet PVE vivo y payloads fijados de Tailscale/OpenTofu | `CERRADO` | I1-03 |
| I1-05 | DPAPI y `gnx-tailscale-enroll` one-shot sólo con `auth_key` | `CERRADO` | I1-02 |
| I1-06 | Descubrimiento cero peers y persistencia controller | `CERRADO` | I1-04, I1-05 |
| I1-07 | `pvecm create` y PVE privado saludable | `CERRADO` | I1-06 |
| I1-08 | OpenTofu local state y LXC seleccionados | `CERRADO` | I1-07 |
| I1-09 | Garage/Forgejo mediante Docker Compose y secretos DPAPI | `EN CURSO` | I1-08 |
| I1-10 | `gnx status --json`, EXE y aceptación real | `NO INICIADO` | I1-01 a I1-09 |

## 9. Desglose de Incremento 2

I2 no comienza hasta que I1 está cerrado.

| ID | Entregable | Estado | Dependencia |
|---|---|---|---|
| I2-01 | Descubrimiento de exactamente un controller entre uno o dos peers host | `NO INICIADO` | I1 cerrado, G0-03 |
| I2-02 | Persistencia member y controller ID/IP | `NO INICIADO` | I2-01 |
| I2-03 | PVE member limpio y preflight de red cluster | `NO INICIADO` | I2-02, G0-04 |
| I2-04 | `pvecm join` protegido | `NO INICIADO` | I2-03, G0-06 |
| I2-05 | Bloqueo verificable de OpenTofu y servicios singleton | `NO INICIADO` | I2-02 |
| I2-06 | Estado quorate de tres nodos, mismo EXE en ambos members y aceptación real | `NO INICIADO` | I2-01 a I2-05 |

## 10. Evidencia

| Fecha | ID | Host | Artefacto o comando | Resultado | Ruta/hash |
|---|---|---|---|---|---|
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `cargo fmt --all -- --check` + `cargo check --workspace` | Formato y compilación correctos | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 - desarrollo sin admin | `cargo build --release -p gnx-host-preflight` | EXE release generado; JSON fail-stop reproduce exit 11 | SHA-256 `FAB9A0CBA8769A2C413592ADE9E5A733B3FB015B856482FEDF400A9552E0EB56` - commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `gnx-host-preflight --format json` | `windows_host` pass, elevación fail, salida JSON única y exit 11 | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · desarrollo sin elevación | `gnx-host-preflight --format yaml` | Uso rechazado por stderr y exit 64 | commit `62d43a4` |
| 2026-07-18 | A-01 | Windows 11 x64 · ejecución elevada | `gnx-host-preflight --format json` | Detectó y corrigió falsos negativos en hipervisor y salida OEM de DISM; Windows, elevación, virtualización, WSL y VMP pasan; fail-stop exit 14 por reinicio pendiente real | SHA-256 `154ADAF4928D3731FF8757DE90F4E4408C734AC0CFE361CC518C72545CBA81B7` · commit `acccf66` |
| 2026-07-18 | A-01 | Windows 11 x64 · ejecución elevada después de reinicio | `gnx-host-preflight --format json` | Seis gates previos pasan; la ruta completa alcanza `podman_msi` y rechaza Podman 6.0.0 con exit 16 frente al pin 6.0.1 | SHA-256 `154ADAF4928D3731FF8757DE90F4E4408C734AC0CFE361CC518C72545CBA81B7` · commit `acccf66` |
| 2026-07-19 | A-02/B-02 | Validación estática del payload `linux/amd64` y máquina WSL x86_64 | Manifest, hashes y prueba `payload_manifest_matches_all_installed_files` | 8 componentes y 12 archivos fijados; el Quadlet PVE delega el mount cgroup v2 escribible al modo systemd de Podman 6.0.1; sin tags mutables, auth keys ni secretos embebidos | manifest SHA-256 `C71133A097770E0A5EBEA50BA46BA01DC252F041089A8613381DDEC4082049F8` · commits `68e7a16`, `5853866`, `5fe7a57` |
| 2026-07-18 | A-03/I1-01 | Windows 11 x64 · ejecución elevada | `gnx-host-preflight prepare-wsl --format json` | Windows, elevación, virtualización, WSL y VMP pasan; no requirió cambios ni reinicio | `gnx-host-preflight.exe` SHA-256 `C24A4411ABF79A549A6484A2403C8C7397D238C47852552C6579627CAC5A21EE` · commit `5410abd` |
| 2026-07-18 | A-03/A-04 | Windows 11 x64 · servicio en consola bajo usuario interactivo | `gnx status --json` contra `\\.\pipe\Quetzalcoatl` | Named Pipe autenticado respondió y RuntimeGate rechazó la identidad incorrecta con `RUNTIME_IDENTITY_INVALID`; no creó ni modificó máquinas. Es una prueba fail-stop, no evidencia del SID de servicio | `gnx-service.exe` SHA-256 `58FDC9F66881C072E1BF5233BE3A365C02C967624E628F92EB8911D393B98F6C`; `gnx.exe` SHA-256 `75643597BEF5F6FC6366F7650AE8CFA633CDB4ED17CFA5BB606147FA03A277C2` · commits `3caff3b`, `fd8293c`, `5853866` |
| 2026-07-18 | A-03/I1-01 | Validación estática WiX 5.0.2 | `build.ps1`; `wix msi validate`; decompile MSI; extract Burn/MSI | MSI válido con 19 archivos, cuenta `NT SERVICE\Quetzalcoatl`, ProductCode fijo y cadena de 5 paquetes. La extracción recuperó la imagen de 249,510,008 bytes con SHA-256 exacto; WSL 2.7.10 y Podman 6.0.1 están embebidos. El EXE aún está sin firma Authenticode | MSI SHA-256 `EAEE5B906700794EB47190EDF7315DA486FA22179DD27296A79DA31573A5FD93`; EXE SHA-256 `1929C0CFFFB7A127D787FE3C98B2A5625281EA8424B25399270308FE6DF906D0` · commits `7860ade`, `7d0cf23`, `5853866`, `11fa964` |
| 2026-07-18 | A-03 | Windows 11 x64 · primera instalación elevada | Burn `install-20260718-215228.log` | PrepareWsl, Podman 6.0.1, ValidateHost y QuetzalcoatlProduct terminaron `0x0`; producto registrado y servicio iniciado. RuntimeGate alcanzó WSL y expuso el defecto tag+digest del resolvedor OCI de Podman 6.0.1; no se degradó a otra imagen | Burn exit `0x0`; corrección `5853866` |
| 2026-07-18 | A-03 | Windows 11 x64 · sustitución controlada del bundle | Burn uninstall + ejecución del EXE final | El bundle anterior se retiró con exit `0x0`, conservando WSL/Podman permanentes. El EXE final verificó sus payloads y PrepareWsl terminó con exit 14 por señal real de reinicio; QuetzalcoatlProduct quedó ausente | bundle final SHA-256 `1929C0CFFFB7A127D787FE3C98B2A5625281EA8424B25399270308FE6DF906D0`; `PendingFileRenameOperations` contiene sólo tres temporales `DEL*.tmp` |
| 2026-07-18 | A-03 | Windows 11 x64 · instalación final elevada | `QuetzalcoatlSetup.exe -quiet -norestart` | Burn terminó exit 0; servicio `Auto` bajo `NT SERVICE\Quetzalcoatl`, CLI, manifest e imagen WSL instalados con hashes exactos | EXE SHA-256 `1929C0CFFFB7A127D787FE3C98B2A5625281EA8424B25399270308FE6DF906D0` |
| 2026-07-19 | A-03 | Windows 11 x64 · reinicio real a las 01:57:21 | SCM + `gnx status --json` | Producto sobrevivió el reboot; servicio volvió automáticamente con SID `S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587`; máquina persistida regresó a `KVM_READY` | A-03 cerrado; bundle SHA-256 `1929C0CFFFB7A127D787FE3C98B2A5625281EA8424B25399270308FE6DF906D0` |
| 2026-07-19 | A-04/G0-01/B-01 | Windows 11 x64 · cuenta virtual instalada | `gnx status --json` después de hot-swap controlado | `PROXMOX_READY`: el binario sólo alcanza ese estado si ambos probes devuelven `KVM_API_VERSION=12;TUN=ready;FUSE=ready` y PVE confirma systemd, cgroup v2, `pve-cluster`, `pvedaemon`, `pveproxy` y `pvesh /version` | `gnx-service.exe` SHA-256 `664F4006A2CBC8E2ACB53FB2560A56E71D201733F687BAAE510577E7B53B4A16`; manifest SHA-256 `C71133A097770E0A5EBEA50BA46BA01DC252F041089A8613381DDEC4082049F8`; commit `5fe7a57` |
| 2026-07-19 | A-05 | Validación estática del servicio y payload final | `cargo clippy -p gnx-service -- -D warnings`; `cargo test -p gnx-service`; sintaxis Dash/Python; hashes | Clippy limpio, 18/18 pruebas y 30/30 payloads. `READY` queda condicionado en código a S3 PUT/GET y Forgejo push/clone, todavía sin atribuirles evidencia viva | manifest SHA-256 `27A44812A1822B7544801965086284B8222711CDC3EA118D33B9D671EA4E60A2` · commit `e5f346a` |
| 2026-07-19 | A-03/I1-01 | Windows 11 x64 · major upgrade elevado | `QuetzalcoatlSetup.exe /install /quiet /norestart`; `wix msi validate`; comparación del payload instalado | Burn detectó 0.1.0, ejecutó PrepareWsl y ValidateHost, instaló MSI 0.1.1, retiró 0.1.0 y terminó `0x0` sin reboot. Binarios y 30 payloads instalados coinciden con el build | EXE SHA-256 `37D7744BFB3D2D2D88D949B0A2A1594A37FB7459B6A8357610243596BE57350B`; MSI `C603D33C35FCE8F89AA138AA72C92E6FEA2C0C2DC81C0C7F09FF41E404E6DE45`; commit `6957709` |
| 2026-07-19 | A-05/B-07 | Windows 11 x64 · entrada elevada negativa | `gnx configure` con password PVE menor al contrato | Rechazo `CONFIGURATION_INVALID` antes de crear `%ProgramData%\Quetzalcoatl.Runtime`; ninguna entrada fue persistida | `gnx.exe` SHA-256 `2817C678654413FB3A3106326EA463B2A9685A6AEEE50A9BF611D73E2C84CFAB`; `gnx-service.exe` `367F3878ECFB20C00B49F9ABA810018AA873AF8B3D79004FE475BCFB378CF63C` |
| 2026-07-19 | A-06 · hosts member | GitHub Actions · dos Dockur Windows 11 simultáneos | Runs `29688898744` y `29689552343`; probe RDP X.224 desde el controller Windows | Ambos runners pasaron KVM API 12, VMX/SVM anidado, TUN, 12 GiB guest y arranque real de Windows; MagicDNS resolvió simultáneamente y ambos endpoints devolvieron X.224 Connection Confirm por la tailnet | `node-2` `100.99.26.47:3389`; `node-3` `100.88.174.82:3389`; Dockur digest `sha256:8cc6f8bc4a60c078141fd3bcf7d2df69ae063a11d98be31a57429afe0dca66da`; workflow commit `3f0379c` |

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
| D-02 | Cero peers host = controller; uno o dos peers con exactamente un controller = member | Cierra la topología a tres nodos con un solo camino member; un cuarto host, cero controllers o múltiples controllers fallan sin elección |
| D-03 | Una `auth_key` con sólo `tag:quetzalcoatl-node`; ese tag posee directamente `tag:quetzalcoatl-service` | Permite identidades host/service exactas sin OAuth ni dos keys; no existe tag controller |
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
| D-21 | Podman Machine OS 6.0.1 se embebe como el layer oficial WSL x86_64 y se verifica antes de crear la máquina | Evita la resolución tag+digest defectuosa y la dependencia de red sin introducir otra imagen, cache mutable o proveedor |
| D-22 | Cada versión MSI usa ProductCode nuevo y conserva UpgradeCode; `MajorUpgrade` corre después de `InstallInitialize` | Permite servicing transaccional y rollback en vez de sustituciones manuales |

## 12. Registro de avance

| Fecha | Cambio | Efecto |
|---|---|---|
| 2026-07-18 | Se cerró la arquitectura y el alcance de dos incrementos | Listo para iniciar A-01 |
| 2026-07-18 | A-01 implementado y contrato fail-stop parcial ejecutado | Permanece `EN CURSO` hasta evidencia elevada completa |
| 2026-07-18 | A-01 alcanzó el gate de reinicio en ejecución elevada real | Se requiere reiniciar el host y reanudar el mismo binario; no se omite ni limpia la señal de Windows |
| 2026-07-18 | A-01 cerró la ruta elevada completa después del reinicio | El Podman 6.0.0 existente fue rechazado correctamente; A-02 queda como único trabajo en curso |
| 2026-07-18 | A-02 cerró B-02 con `runtime payload v1` verificable | A-03 inicia sobre un único conjunto de digests y archivos; los snippets quedan sólo como referencia de topología |
| 2026-07-18 | A-03 produjo y validó estáticamente el primer setup completo | A-03 permanece abierto hasta ejecutar el bundle elevado, comprobar servicio/CLI y demostrar el mismo SID después de reiniciar |
| 2026-07-18 | La primera instalación elevada reveló el defecto tag+digest de Podman 6.0.1 | La imagen WSL oficial quedó fijada y embebida; no se añadió fallback ni resolución por red |
| 2026-07-18 | El bundle anterior se desinstaló y el EXE final alcanzó el gate de reinicio real | WSL 2.7.10 y Podman 6.0.1 permanecen; producto/servicio están ausentes hasta reiniciar y reanudar A-03 |
| 2026-07-19 | A-03 cerró después de instalación final y reboot real | Servicio/SID, perfil, máquina y CLI persistieron; A-04 pasó a ser el único trabajo activo |
| 2026-07-19 | A-04 cerró G0-01 y B-01 con `PROXMOX_READY` bajo la cuenta virtual | A-05 inicia por DPAPI/Tailscale; G0-02 permanece abierto hasta probar PVE después de reboot |
| 2026-07-19 | El bundle 0.1.1 actualizó 0.1.0 mediante major upgrade real | Producto, servicio y 30 payloads instalados coinciden con el build; A-05 continúa por la ACL y entrada DPAPI |

Al actualizar este archivo:

1. cambiar el estado de la tarea activa;
2. registrar evidencia y cerrar o actualizar sólo los stoppers afectados;
3. marcar la siguiente tarea como `EN CURSO`;
4. añadir una línea al registro;
5. no agregar backlog futuro.
