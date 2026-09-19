# Auditoría 360 — GNX 0.3.1

**Fecha:** 2026-10-03  
**Alcance:** árbol de trabajo actual, documentación normativa, Rust, empaquetado y pruebas estáticas. No es una certificación de una instalación real ni una aceptación de release.

## Veredicto ejecutivo

**Estado: BLOQUEADO / no liberable.** El repositorio contiene una base de implementación real y relativamente estructurada (núcleo Rust, adaptadores Linux/Windows, estado transaccional, protocolo de pipe y empaquetado), pero no evidencia ejecutable de los resultados que definen el producto: G0–G6, cliente remoto autorizado, recuperación y paridad Windows/Linux.

La afirmación prudente es: **slice funcional en código, laboratorio y release incompletos; no hay base para declarar GNX 0.3.1 listo, ni para afirmar que noVNC/Dockur ha validado el producto.**

La ejecución local de `cargo fmt --all -- --check && cargo test --locked` falló antes de las pruebas: el target `x86_64-pc-windows-msvc` invoca `/usr/bin/link.exe` (GNU link) como `link.exe`, no el linker MSVC. `tests/runtime_smoke.py` tampoco se pudo ejecutar porque no hay intérprete Python funcional (el alias de Microsoft Store devolvió error). Estos gates se registran como **BLOCKED**, no como pass.

## Qué existe realmente

| Área | Evidencia en árbol | Nivel | Conclusión |
|---|---|---:|---|
| Contrato | `README.md`, `docs/01-*` a `07-*`, JSON y códigos en `src/report.rs` | Medio | El objetivo, cuatro operaciones y gates están bien descritos. |
| Núcleo de aplicación | `src/app/*`, `src/domain/*`, `src/port/*` | Medio | Hay separación de puertos/adaptadores y apply con lock, staging, promoción y rollback. |
| Linux | `src/adapter/linux.rs`, `runtime/*` | Medio-bajo | Hay código de Podman/systemd/Tailscale/CoreDNS/Caddy/Compute, pero no prueba E2E en host. |
| Windows | `src/bin/gnx-{service,setup}.rs`, `src/adapter/windows/*`, `packaging/windows/*` | Medio | Hay implementación considerable de setup/broker/cuenta; bootstrap WSL y aceptación limpia no están probados. |
| Pruebas | `tests/*.rs`, una integración ignorada en `src/adapter/linux.rs` | Bajo para aceptación | Principalmente dobles/fakes, parsing, framing y checks de texto; no cubren G0–G6. |
| Release | `runtime/release.toml`, scripts build | Bajo | Imágenes llevan digest, pero no hay firma/atestado ni candidate sellado verificable. |
| noVNC/Dockur | Sólo narrativa histórica en `docs/05-acceptance.md` | Ausente | No hay definición reproducible de lab, automatización ni prueba noVNC en el árbol. |
| CI/evidencia | No se encontró workflow/Makefile/compose/noVNC | Ausente | No hay pipeline que imponga los gates ni índice de evidencia actual. |

## Hallazgos críticos

### C1 — La aceptación declarada no es demostrable

`README.md` dice que la rama contiene un “integrated 0.3.1 functional slice” cubierto por pruebas y gates; `docs/05-acceptance.md` y `docs/07-implementation.md` reconocen a la vez que el test local MSVC está bloqueado y que la aceptación host sigue pendiente. Los G0–G6 requieren host Linux/Windows y cliente remoto; el árbol no contiene evidencia actual de ninguna corrida.

**Riesgo:** confundir cobertura de unit/static tests con aceptación de producto.  
**Decisión:** no promocionar ni usar `READY` como evidencia de release hasta adjuntar el índice de G0–G6.

### C2 — El release no está autenticado, aunque la documentación lo exige

`src/adapter/release.rs` valida formato/digests de referencias embebidas y `runtime/release.toml` fija digests de imágenes. Eso no autentica un manifiesto de release. `packaging/windows/build.ps1` genera un `manifest.json` y muestra su SHA-256, mientras que `install.ps1` compara un hash suministrado por el operador. No hay firma, clave/identidad de confianza, attestation, SBOM ni política de rotación implementada.

Además, `packaging/windows/runtime.lock.json` está explícitamente `unsealed` y tiene `rootfs`, `bundle` nulos.

**Riesgo:** un hash introducido fuera de un canal autenticado no demuestra procedencia; no existe candidate Windows completo.  
**Acción:** introducir manifest canónico firmado (por ejemplo, Sigstore/cosign o Ed25519 con raíz offline), verificación en setup, SBOM y attestation; bloquear `--provision` para manifests sin firma válida.

### C3 — La prueba de Control no prueba el requisito remoto

`Linux::control_health` en `src/adapter/linux.rs` hace `nsenter` en el namespace de Access y usa `curl --resolve`; el mismo adaptador comprueba DNS localmente. Eso es útil como sonda interna, pero no demuestra:

- resolución desde el resolver normal de un cliente autorizado;
- alcance privado real desde otro host;
- rechazo desde un cliente no autorizado;
- aislamiento de puertos desde red remota.

Es exactamente la sustitución que `docs/05-acceptance.md` prohíbe.

**Acción:** un runner cliente externo debe ejecutar consultas DNS del sistema, TLS con CA importada, login Compute, TCP negativo a puertos internos y pruebas de autorización. Sus resultados sanitizados son evidencia G2/G3; nunca sustituirlos por noVNC, loopback o `nsenter`.

### C4 — noVNC/Dockur no es una implementación de pruebas

El repositorio no contiene `docker-compose`, Containerfile, script de arranque, script guest, definición de red, snapshot, schema de resultados ni colector de evidencia para noVNC/Dockur. La documentación conserva hechos históricos y dice expresamente que noVNC sólo prueba la consola.

**Acción:** tratar noVNC exclusivamente como consola diagnóstica de una VM Windows desechable. La aceptación se automatiza dentro del guest y desde cliente remoto; noVNC no puede ser un gate de `READY`.

### C5 — El flujo de runtime depende de capacidades no validadas ni declaradas como matriz

Linux exige root, systemd, cgroup v2, Podman, `/dev/kvm`, `/dev/fuse`, `/dev/net/tun`, 32 GiB libres, curl y openssl. El Compute ejecuta `dockurr/proxmox` privilegiado y usa KVM. La matriz de plataformas, versiones mínimas de kernel/Podman/WSL, nested virtualization y política de recursos no está declarada como artefacto verificable.

**Acción:** publicar una matriz versionada y hacer que `doctor` compruebe cada requisito, capacidad y versión de forma explícita. Un host virtualizado debe exponer VT-x/AMD-V anidado y KVM al runtime; sin ello el gate es BLOCKED, no degradado.

### C6 — La recuperación y último válido son más débiles que el contrato

`apply` mantiene candidate/previous y rollback, pero `Linux::restore` llama `reconcile(c)` sin secreto. Si la restauración necesita secretos que no estén ya persistidos, puede fallar; una restauración de Access no puede reenrolar sin key. Tampoco hay pruebas de corte de energía/reboot que comprueben identidad, CA y datos Compute persistentes.

**Acción:** diseñar y probar recovery con secretos persistidos sólo donde corresponda, idempotencia tras reboot y journaling por recurso; añadir inyección de fallo tras cada fase y prueba G4/G5 en host real.

## Brechas importantes

1. **E2E ausente:** no hay prueba automatizada para G0–G6, ni runner de cliente remoto.
2. **Seguridad de supply chain:** falta firma, identidad de firmante, SBOM, provenance, escaneo y política de vulnerabilidades.
3. **Actualización:** no hay implementación de update/rollback de release, migración de estado ni política de compatibilidad de schema.
4. **Observabilidad:** faltan esquema de evidencia, IDs de ejecución, retención, métricas, logs estructurados sanitizados y alertas.
5. **Operación:** no hay SLO, backup/restore probado de identidad/CA/datos Compute, RPO/RTO ni procedimiento de rotación/revocación de CA/enrolamiento.
6. **Red:** el transporte está concretamente acoplado a Tailscale (`Linux::identity` y `access_start`), aunque los docs presentan el proveedor como decisión abierta. Debe decidirse y declararse como release fact antes de release.
7. **Aislamiento:** Compute se inicia privilegiado. Hace falta threat model, justificación, hardening de Podman/SELinux y test de exposición de puertos.
8. **Calidad de tests:** las pruebas actuales usan `Fake`, aserciones de texto y una integración `#[ignore]`; no detectarán regresión de runtime, Windows, redes o certificados.
9. **Automatización:** no hay CI detectable que haga fmt, clippy, test, build reproducible, auditoría de dependencias, pruebas de artefactos ni publicación de evidencia.
10. **Estado del árbol:** hay cambios no confirmados: README modificado, 26 documentos eliminados y siete nuevos no trackeados. La auditoría no puede asociar el contenido actual a un commit limpio/reproducible.

## Afirmaciones que deben corregirse o condicionarse

No se atribuye intención; son afirmaciones **no sustentadas** por la evidencia disponible.

| Afirmación | Motivo | Redacción honesta |
|---|---|---|
| “integrated 0.3.1 functional slice” | El build/test local está bloqueado y no hay E2E G0–G6. | “Implementación parcial; aceptación de host pendiente.” |
| “covered by repository tests and static gates” | Los tests son mayormente unit/fake y no prueban el resultado remoto. | “Hay cobertura de contrato y estado; runtime E2E pendiente.” |
| “release autenticado” | Sólo se verifica un hash entregado; no hay firma/atestado. | “Artefactos con hash; autenticación de release aún no implementada.” |
| “Windows release contiene…” | `runtime.lock.json` está sin sellar y con paths nulos. | “Diseño/flujo de empaquetado existente, candidate Windows no sellado.” |
| cualquier éxito basado en noVNC | noVNC sólo expone consola. | “noVNC alcanzable; guest/producto sin verificar.” |

## Flujo sostenible para pruebas 360 en host virtualizado dedicado

### Topología mínima

1. **Host de virtualización dedicado:** Linux soportado, KVM habilitado, almacenamiento rápido y aislado, snapshots; sin secretos en imágenes/snapshots. Para Dockur/Windows debe tener nested virtualization disponible para el guest y permitir `/dev/kvm` y `/dev/net/tun` sólo al lab.
2. **VM Windows desechable:** Windows 11 soportado; no es el host de confianza ni el cliente remoto. Corre el bundle GNX y publica noVNC sólo a loopback/VPN administrativo.
3. **Runtime GNX/WSL dentro del guest:** objeto bajo prueba; volumen/state propio y nunca montajes de `legacy`, ProgramData productivo o secretos del host.
4. **VM Linux cliente autorizado:** entra a la red privada de prueba y ejecuta G2/G3 como usuario remoto real.
5. **VM Linux cliente no autorizado:** prueba denegación de transporte/puertos.
6. **Runner/controlador efímero:** construye artefactos, crea staging de sólo hashes públicos, lanza checks y reúne JSON sanitizado. No entra en el runtime GNX.

### Secuencia de una corrida

1. Crear candidate desde checkout limpio; registrar commit, lockfiles, hashes, SBOM y firma de manifest.
2. Validar firma/digests antes de encender el guest. Si no hay firma, terminar **FAILED: RELEASE_UNAUTHENTICATED**.
3. Crear VM/volúmenes nuevos con ID único; habilitar nested virtualization sólo para esta VM. Conservar snapshot base inmutable.
4. Exponer noVNC en loopback o red administrativa privada; generar URL/token fuera de Git, argv, logs y evidencia. No capturar pantalla con datos sensibles.
5. Copiar al guest únicamente bundle autenticado y scripts de prueba sin secretos. El guest escribe a share temporal sólo JSON pequeño: `run_id`, `phase`, `state`, `code`, `exit`, hashes y timestamps.
6. Ejecutar `gnx-setup --check`, luego `--provision`; exigir los estados especificados, ACL/cuenta/pipe y rechazos de conflictos/reparse. Aún no declarar READY.
7. Completar bootstrap WSL explícito y ejecutar `doctor`, `plan`, `apply`, `status`. Los secretos se inyectan por canal protegido de CI/runner, nunca por noVNC ni share.
8. Desde cliente autorizado: resolver `compute.gnx` usando resolver normal, validar cadena/hostname con CA importada deliberadamente, autenticar Compute y verificar almacenamiento.
9. Desde cliente no autorizado y desde red externa: comprobar denegación de entrypoints y de puertos Compute/Control internos. Probar DNS fuera de `.gnx` y nombre no declarado.
10. Ejecutar G4: `plan` sin mutación, apply idempotente, candidate corrupto/insano no promociona, opcional caído no degrada capacidades requeridas.
11. Ejecutar G5: restart de servicio, reboot WSL y reboot guest; comparar identidad, revision last-valid y witness de storage.
12. Ejecutar G6: mismas fixtures/operaciones/códigos Linux y Windows; comparar JSON canónico.
13. Recoger evidencia sanitizada, apagar/limpiar recursos o registrar retención explícita. Promoción sólo si el índice completo es pass.

### Gates automatizables obligatorios

- `fmt`, `clippy -D warnings`, tests unit/property/fuzz de framing y config.
- build Linux/Windows desde checkout limpio con toolchains fijados.
- manifest firmado + verificación independiente + SBOM/licencias.
- tests de instalación Windows desechable y WSL bootstrap.
- G0–G6 completos, con matriz versionada y cliente remoto.
- escaneo de secretos en source, artefactos y evidencia; no publicar logs crudos.
- destrucción o retención declarada de VM, volúmenes y CA de laboratorio.

## Plan priorizado

### P0: impedir falsos positivos

1. Corregir el toolchain MSVC: instalar/seleccionar Build Tools y asegurar que `link.exe` sea el linker MSVC; no usar `/usr/bin/link.exe` para target MSVC.
2. Instalar un Python real o convertir `runtime_smoke.py` en test Rust/runner con runtime fijado.
3. Cambiar README/status para marcar G0–G6 y host acceptance como BLOCKED hasta que haya evidencia.
4. Definir y aplicar manifest firmado; mantener `runtime.lock.json` sin sellar como bloqueo duro.
5. Limpiar o confirmar el árbol antes de cualquier evidence run.

### P1: laboratorio reproducible

1. Añadir infraestructura versionada para host/VM, sin secretos: definición de VM, red aislada, recursos, cleanup y schema de resultados.
2. Añadir scripts guest y clientes remoto autorizado/no autorizado; noVNC queda opcional para diagnosis.
3. Completar bootstrap WSL y testear setup, ACL, pipe, recovery y reboot.
4. Declarar matriz soportada y checks exactos en `doctor`.

### P2: calidad de producto y futuro

1. Sustituir adapters acoplados/decisiones abiertas por release facts sellados.
2. Crear update/rollback y compatibilidad de schemas con pruebas de migración.
3. Implementar backup/restore, rotación CA/enrollment, threat model y hardening del contenedor privilegiado.
4. CI con artefactos retenidos y evidencia indexada por candidate/run ID.

## Evidencia de esta auditoría

- Inventario de código/runtime/packaging/tests leído del árbol actual.
- `cargo fmt --all -- --check && cargo test --locked`: **BLOCKED**, fallo de linker antes de tests.
- `python tests/runtime_smoke.py`: **BLOCKED**, Python no disponible.
- No se encontraron archivos de noVNC/VNC/Docker Compose/CI workflow dentro del alcance inspeccionado.
- No se modificó código de producto ni `legacy` durante la auditoría.

## Criterio de salida de la auditoría

Este informe sólo puede pasar a “implementación validada” cuando existe un candidate limpio y firmado, y un índice reproducible demuestra G0–G6 en la matriz declarada, incluyendo el cliente remoto. La disponibilidad de un contenedor, una consola noVNC, un proceso o una página HTTP no cumple ese criterio.
