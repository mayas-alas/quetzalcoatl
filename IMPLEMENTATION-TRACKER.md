# Quetzalcoatl Next — tracker de implementación 1.x

**Actualizado:** 2026-08-30  
**Fuente de arquitectura:** `QUETZALCOATL-NEXT-LEAN-ARCHITECTURE-PROPOSAL.md`  
**Estado:** primera base greenfield en construcción  
**Targets:** Windows x86_64 y Linux x86_64

## Meta operativa

Entregar un único producto `gnx` que prepare Windows o Linux y converja el mismo
runtime Linux dentro de Podman Machine `quetzalcoatl`. Headscale es el único
control plane; Docktail reconcilia servicios por celda; Proxmox y OpenTofu crean
LXC cerrados; systemd y Quadlets gobiernan el lifecycle.

`READY` significa evidencia completa. Un componente pendiente o incompatible no
se degrada silenciosamente ni cambia el controller configurado.

## Decisiones vigentes

- [x] Línea 1.x nueva; no migra ni lee state 0.x.
- [x] Un crate Rust principal y una experiencia pública `gnx`.
- [x] Headscale externo y operado fuera de GNX.
- [x] Controllers válidos: `https://headscale.node.gnx` y
  `https://controlplane.node.gnx`.
- [x] Resolución DNS y TLS del dominio `.gnx` son infraestructura ya cubierta y
  no se implementan dentro de GNX.
- [x] Windows usa identidad dedicada dueña de Podman y Podman Machine; el usuario
  host opera únicamente mediante `gnx`.
- [x] Cada celda mantiene su propio `tailscaled`; Docktail lo utiliza pero no lo
  sustituye.
- [x] Docktail y workloads se expresan con Quadlets dentro de cada celda.
- [x] Backup y recovery quedan fuera del alcance actual por decisión del producto.
- [x] `update`, `repair` y `uninstall` no destructivo permanecen dentro del alcance.

## Correcciones a la propuesta fuente

1. Las secciones 40, 52/Fase 4, 56 y 57 incluyen backup o recovery. Para esta
   implementación se consideran fuera de scope. No se crearán interfaces vacías
   ni placeholders que sugieran soporte.
2. Docktail no reemplaza al cliente Tailscale. La documentación vigente indica
   que anuncia servicios mediante el daemon local y requiere su socket. El LXC
   conserva `tailscaled`; las aplicaciones no reciben identidades individuales.
3. El gate Docktail + Headscale no está disponible para GA: Headscale mantiene
   abierto el soporte de Tailscale Services. Ningún recorrido puede reportarlo
   como compatible por inferencia.
4. El binario ELF Linux de la primera pasada es un artefacto de desarrollo. El
   artefacto distribuible continúa siendo AppImage y requiere su propio gate.

## Leyenda

- `[x]` terminado y verificado localmente.
- `[ ]` pendiente.
- `[BLOCKED]` depende de evidencia externa o hardware real.
- `[DECISION]` requiere una decisión de producto antes de implementar.

## P0 — primera base y binarios de desarrollo

- [x] Crear crate Rust greenfield sin código legacy.
- [x] Fijar toolchain y `Cargo.lock`.
- [x] Implementar contrato CLI: `install`, `init`, `status`, `doctor`, `repair`,
  `update`, `uninstall`, `version`.
- [x] Ocultar los entrypoints internos `__service`, `__tray`, `__resume`.
- [x] Implementar `status` y `doctor` sin mutaciones.
- [x] Hacer fallar las operaciones aún no implementadas con código estable y sin
  fingir éxito.
- [x] Parsear TOML con schema 1 y rechazo de campos desconocidos.
- [x] Exigir Podman Machine `quetzalcoatl`.
- [x] Validar controller HTTPS/DNS/443 sin credentials, path, query o fragment.
- [x] Rechazar IP literal sin aplicar una política especial por marca/dominio.
- [x] Probar los dos nombres `.gnx` requeridos.
- [x] Documentar dependencias directas.
- [x] Ejecutar `fmt`, `clippy` y tests en Windows.
- [x] Construir `dist/gnx-windows-x86_64.exe` y verificar `version`.
- [x] Construir `dist/gnx-linux-x86_64` estático y verificarlo dentro de Linux.
- [x] Consolidar `SHA256SUMS` y metadata de desarrollo.
- [ ] Empaquetar y probar `dist/gnx-x86_64.AppImage`.

### Acceptance P0

- [x] Ambos binarios muestran la misma versión y ayuda pública.
- [x] Ambos aceptan `headscale.node.gnx` y `controlplane.node.gnx`.
- [x] Ambos validan el contrato técnico sin blacklist de controllers.
- [x] `status --json` nunca reporta `ready` en la base incompleta.
- [x] `init` falla sin mutar y remite a este tracker.
- [x] Los checksums coinciden con los artefactos entregados.

### Evidencia P0

- Toolchain: Rust `1.98.0`.
- Tests: 11 unitarios aprobados en Windows y Linux/musl.
- Estático: ELF x86_64 static PIE, stripped.
- Exit codes comprobados en ambos targets: `init=3` y doctor incompleto `=4`.
- SHA-256 Windows: `cdc340cc0ce6399167a9172443d0d4407a600b16bd4bb68c65602a15b8f6b434`.
- SHA-256 Linux: `cf61895033627e6621911fd982330dfdba0e667c5ab0bb8e1fb6299e0e0bc46e`.

## P1 — factibilidad soberana y matriz fijada

- [ ] Fijar versiones estables de Headscale, Tailscale client y Docktail.
- [ ] Fijar Podman, Podman Machine image, OpenTofu y providers.
- [ ] Publicar matriz mínima de Windows/Linux probados.
- [ ] Implementar preflight TLS contra el controller configurado.
- [ ] Verificar que enrolamiento utiliza exactamente el controller configurado.
- [BLOCKED] Ejecutar `MESH-SVC-01` extremo a extremo contra Headscale fijado.
- [BLOCKED] Aprobar Docktail sólo si Headscale Services, policy, DNS, drain,
  restart y rotación pasan físicamente.
- [DECISION] Si falla `MESH-SVC-01`: esperar upstream, contribuir upstream,
  sustituir Docktail o cambiar el modelo de exposición. GNX no elige en código.

## P2 — vertical Linux

- [ ] Preflight Linux x86_64 con distro/kernel explícitos.
- [ ] Podman Machine `quetzalcoatl` con systemd, cgroup v2, KVM y TUN.
- [ ] Detectar y fallar `LEGACY_CONFLICT`/`MACHINE_NAME_CONFLICT` antes de mutar.
- [ ] Instalar `tailscaled` de sistema en la celda runtime.
- [ ] Enrolar con pre-auth key efímera y `--login-server` fijado.
- [ ] Generar y activar Quadlet Docktail por digest.
- [ ] Desplegar un workload mínimo por digest.
- [ ] Verificar health local, identidad mesh y ruta direct/relay.
- [ ] Producir AppImage, probar FUSE y `--appimage-extract-and-run`.

## P3 — Proxmox, OpenTofu y primer LXC

- [ ] Ejecutar Proxmox mediante Quadlet en la celda runtime.
- [ ] OpenTofu one-shot sin `local-exec`, `remote-exec` ni provisioners.
- [ ] Crear un LXC con VMID y límites asignados por GNX.
- [ ] Ejecutar bootstrap fijo, repository-owned y acotado por stdin.
- [ ] Instalar Podman y cliente Tailscale fijados dentro del LXC.
- [ ] Arrancar `tailscaled`, Podman socket, Docktail y workload por systemd.
- [ ] Confirmar que el socket Podman no cruza fronteras de celda.
- [ ] Confirmar endpoint privado y policy deny-by-default.

## P4 — Windows e identidad dedicada

- [ ] Implementar preflight y elevación nativos.
- [BLOCKED] Probar físicamente qué identidad administrada soporta perfil,
  WSL/Podman Machine y ownership sin logon interactivo.
- [ ] Crear ACL de mínimos privilegios para binario, state, journal y secretos.
- [ ] Registrar Windows Service nativo con entrypoint `__service`.
- [ ] Implementar Named Pipe versionado con operaciones cerradas.
- [ ] Implementar reboot/resume con journal monotónico.
- [ ] Hacer que la identidad dedicada sea dueña de Podman Machine.
- [ ] Confirmar que el usuario host no recibe socket, secretos ni acceso ordinario
  a la identidad dedicada.
- [ ] Implementar tray nativo como cliente del servicio, sin state propio.

## P5 — mantenimiento dentro de alcance

- [ ] `update` explícito, verificado, atómico y con rollback del binario.
- [ ] `repair` idempotente sin rotar identidad ni destruir datos.
- [ ] `uninstall` retira integración GNX y conserva máquinas, LXC y volúmenes.
- [ ] Firma QA, Authenticode y evidencia de integridad.
- [ ] Logs redactados, límites de salida y errores accionables.

No se incluyen tareas de backup, restore, retención ni recuperación ante desastre.

## Gates que nunca se convierten en warnings

| Gate | Estado | Evidencia requerida |
|---|---|---|
| `GREENFIELD-01` | Activo | Cero lectura/adopción de 0.x. |
| `CTRL-01` | Parcial | HTTPS válido para ambos aliases y controller efectivo igual al configurado. |
| `MESH-SVC-01` | BLOCKED | Docktail + Headscale Services extremo a extremo. |
| `WIN-ID-01` | BLOCKED | Identidad dedicada probada en Windows físico. |
| `LINUX-PKG-01` | Pendiente | AppImage ejecutado con FUSE y fallback. |
| `CELL-ISO-01` | Pendiente | Socket, identidad y secretos aislados por celda. |
| `SUPPLY-01` | Pendiente | Digests, checksums, firmas y SBOM verificables. |

## Referencias verificadas el 2026-08-30

- Docktail requiere socket Docker-compatible y socket del daemon Tailscale:
  https://docktail.org/docs/
- Gap abierto de Tailscale Services en Headscale:
  https://github.com/juanfont/headscale/issues/2845
- Matriz de features estable de Headscale:
  https://headscale.net/stable/about/features/

## Regla de avance

Antes de marcar una casilla:

1. ejecutar la prueba correspondiente;
2. conservar comando, versión y resultado reproducible;
3. verificar que no cambió el controller configurado ni se filtraron secretos;
4. marcar `READY` únicamente cuando todos los gates del recorrido estén cerrados.

Si una tarea no resuelve un fallo real del recorrido aprobado, no entra.
