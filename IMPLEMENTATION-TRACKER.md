# Quetzalcoatl Next — MVP tracker

**Corte:** 2026-08-31

**Objetivo:** EXE Windows y AppImage Linux que instalan `gnx`, preparan el host y
reconvergen la topología después de reinicios.

**Fuera de alcance:** backup, restore, disaster recovery y migración 0.x.

Leyenda: `[x]` implementado y verificado localmente; `[ ]` pendiente;
`[PHYSICAL]` requiere host limpio, hardware o servicios reales.

## Contrato de producto

- [x] Greenfield; no adopta state ni recursos 0.x.
- [x] Abrir EXE/AppImage sin argumentos inicia el instalador.
- [x] No existe un subcomando público de instalación.
- [x] `gnx` queda en `PATH`; sin argumentos muestra ayuda.
- [x] CLI: `init`, `status`, `doctor`, `logs`, `repair`, `update`, `uninstall`,
  `version`.
- [x] Controllers HTTPS configurables, incluidos los dos aliases `.gnx`, sin
  política de rechazo por marca.
- [x] `gnx init --controller-address <IP>` persiste el bootstrap, administra sólo
  el bloque GNX de `hosts` y lo propaga a los Quadlets del runtime y del LXC.
- [x] Desinstalación retira Podman CLI y conserva máquinas, LXC, volúmenes,
  configuración y state.

## Host e instalación

- [x] Windows: UAC, WSL automático, Podman MSI verificado, PATH de máquina,
  reboot/resume con journal y Windows Service automático.
- [x] Windows: cuenta local `gnx-runtime` con perfil WSL propio, contraseña no
  persistida, logon de servicio y denegación de logon interactivo/remoto/red.
- [x] Windows: el proceso original espera UAC, inicia tray inmediatamente,
  registra tray al logon y difunde el cambio de `PATH` a Explorer.
- [x] Windows: JSONL persistente y `gnx logs`; checkpoints, servicio, runtime,
  tray y errores quedan trazables.
- [x] Windows: recuperación del servicio con reintentos SCM a 10/30/60 segundos.
- [x] Linux: sudo automático y soporte de instalación con apt, dnf o pacman.
- [x] Linux: Podman, QEMU y FUSE automáticos, CLI en `/usr/local/bin` y
  `gnx-host.service` habilitado.
- [x] `repair` reconverge sin destruir datos; `update` exige artefacto local y
  SHA-256.
- [PHYSICAL] `WIN-ID-01`: probar perfil WSL/Podman de la cuenta local dedicada en un
  Windows limpio y confirmar que el usuario host no posee ese perfil.
- [PHYSICAL] `LINUX-INSTALL-01`: probar AppImage con FUSE, elevación y reboot en
  una distro limpia por cada familia de paquetes.

## Runtime soberano

- [x] Podman Machine fija `quetzalcoatl`, rootful, 4 CPU, 8 GiB y 100 GiB.
- [x] Marcador de propiedad obligatorio; una máquina homónima ajena falla antes
  de arrancar o recibir archivos.
- [x] Preparación de máquina separada del gate controller: una caída DNS/TLS no
  marca la máquina como fallida ni impide reintentos observables.
- [x] systemd y Quadlets fijados para `tailscaled`, Docktail y Dockur Proxmox.
- [x] Tailscale y Docktail usan imágenes por digest y sockets locales por celda.
- [x] tailscaled conserva exactamente `controlplane.node.gnx` como
  `--login-server`; Docktail consume su socket local y no configura un control
  plane paralelo.
- [x] El gate del controller consulta `/health` de Headscale por HTTPS y exige
  respuesta 2xx antes de enrolar.
- [x] Dockur Proxmox usa KVM/FUSE, persistencia y healthcheck.
- [x] OpenTofu `1.12.6` y provider `bpg/proxmox` `0.111.1` fijados.
- [x] OpenTofu se ejecuta exclusivamente en LXC 200 `gnx-infra-runner`.
- [x] Token API y state de OpenTofu permanecen root-only dentro del runner.
- [x] Módulo validado: crea LXC 201 sin provisioners y usa Ubuntu inmutable con
  SHA-256.
- [x] Bootstrap fijo entrega Podman, tailscaled y Docktail al workload LXC.
- [PHYSICAL] `KVM-LXC-01`: ejecutar Dockur Proxmox, runner, `apply` y workload en
  hardware con nested virtualization.
- [PHYSICAL] `MESH-AUTH-01`: entregar pre-auth keys sin persistirlas en config,
  journal, state o argumentos.
- [PHYSICAL] `MESH-BOOTSTRAP-01`: usar la IP real de Headscale, confirmar ambos
  aliases en Windows/Podman Machine/LXC y validar la cadena TLS en las tres
  fronteras.
- [PHYSICAL] `MESH-SVC-01`: validar Docktail Services extremo a extremo contra la
  versión Headscale elegida.

## Build y evidencia

- [x] Rust 1.98.0, `Cargo.lock` y dependencias externas fijadas.
- [x] `cargo fmt`, tests y Clippy sin warnings.
- [x] HCL: `fmt`, `init -backend=false -lockfile=readonly` y `validate`.
- [x] Scripts shell de guest aceptados por `bash -n`.
- [x] EXE release construido; icono, metadata, `version` y tray comprobados.
- [x] ELF y AppImage reconstruidos; `version`, ayuda, controller y rechazo del
  subcomando inexistente comprobados dentro de Linux.
- [x] `finalize-development-release.ps1` ejecutado y checksums comprobados desde
  Linux.
- [ ] Firma Authenticode/AppImage y SBOM para un release público.

Evidencia local:

- Tests: 39 Windows, sin fallos; Clippy sin warnings.
- Windows observado: CLI, tray, servicio dedicado y Podman Machine listos. La
  instalación presente aún está bloqueada en DNS porque falta introducir la IP
  real del Headscale; el binario final debe reinstalarse y repetirse la aceptación.
- Windows EXE nuevo: `102fce1c2409885ba3de4f3014f6c1a0f4220932bce927c1a48366711f516ddd`
  (3,176,960 bytes); `version` y contrato `init --help` verificados.
- Linux ELF: `3c44643cfa54713607bec24b1acdfeffb096b317bb7e097dd075f48a6d77143e`.
- Linux AppImage: `a8bce8f1c95e6ec7e54f8cc4cc6b8401a923a18ddea553fe6180d57b5661ff4d`.
- Quadlets runtime/guest aceptados por el generador; HCL válido con lock read-only.

## Riesgo de frontera aceptado para este MVP

El LXC runner evita que OpenTofu y su token estén disponibles en el uso ordinario
de la Podman Machine. No protege contra una toma de `root` de esa máquina porque
ella hospeda el Proxmox privilegiado; aislamiento contra ese atacante exige un
Proxmox externo. GNX documenta esta limitación y no la reporta como seguridad
cerrada.

## Commits de seguimiento

- `771f378` — baseline greenfield.
- `159b088` — OpenTofu aislado en LXC dedicado.
- `d3a0ee8` — branding de instalador y tray Windows.
- `5a5dbc7` — AppImage y metadata de release portables.
- `c7f3d85` — identidad Windows aislada, tray inmediato y logs persistentes.
- `64c12dc` — enrolamiento Headscale y bootstrap soberano.
- `2c2007b` — gates físicos antes de reportar `READY`.
- `dfde9d4` — secreto mesh transitorio endurecido.
- `ec6eda7` — configuración automática de resolución y `/health` Headscale.
