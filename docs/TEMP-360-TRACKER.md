# TEMP 360 Tracker — alcance mínimo sin crecer arquitectura

## Corte de trabajo — 2026-09-19

| Área | Estado | Evidencia / siguiente acción |
|---|---|---|
| Base | PASS | `cargo fmt --all -- --check`; `cargo test --locked` (18+2+ integración, 1 ignorada) |
| Artefactos Linux | PASS local | `gnx-linux`, bundle y `gnx-linux.run` generados con `gnx-node`; el `.run` instala la misma capa Wide Linux |
| Candidate autenticado | BLOCKED | falta rootfs verificado y firma de producción; `runtime.lock.json` sigue `unsealed` |
| Host Windows / reboot | BLOCKED | no hay PowerShell elevado disponible; no se declara instalación ni resiliencia |
| Residuos del host | PASS preflight observado | ausentes las raíces GNX/Quetzalcoatl versionadas inspeccionadas; no se borró nada |
| Terminología | IN PROGRESS | docs activos usan **Wide Linux**; la tecnología host queda sólo como detalle de implementación |

No se modifica `legacy`; el análisis delegado quedó documentado y un gate bloqueado permanece bloqueado.

## Resumen taxonomía 360 — 2026-09-19

Se agregó `docs/TAXONOMY-PROPOSAL.md` como propuesta no normativa para separar vocabulario de producto, capas, estados públicos, fases de lifecycle, códigos estables, artefactos y evidencia Wide Linux. Hallazgos principales: `compute.gnx` es la ruta operativa requerida; `app.gnx` se conserva como superficie propia HTML/CSS/JS de presentación, no como capacidad adicional; `src/cli.rs` conserva una superficie no alineada con el contrato público actual; `install-host.ps1`, `install.ps1` y `gnx-setup.exe` deben nombrarse como flujos distintos; y Windows/rootfs/firma permanecen bloqueados hasta una prueba elevada con manifest sellado y rootfs verificado. Prioridad inmediata: no declarar READY desde provisioning, build local o bundle `unsealed`; registrar host/reboot/residuos como gates separados.

> Tracker temporal. No es especificación nueva. No autoriza comandos nuevos ni cambios de arquitectura. No incluir secretos. `legacy` sólo se consulta como archivo histórico; no se modifica.

## Regla del 360

Validar lo que ya existe o está documentado. Si algo pedido no existe, se marca como **gap** y no se inventa interfaz.

## Interfaz GNX existente a usar en pruebas

Comandos públicos actuales observados en `src/cli.rs`:

```text
gnx doctor --config <file>
gnx install --config <file> --release <file>
gnx connect --config <file> --setup-key-file <file>
gnx access configure
gnx access apply
gnx access dns
gnx credentials control
gnx credentials compute
gnx-setup.exe --check ...
gnx-setup.exe --provision ...
```

No asumir comandos como `gnx service status`, `gnx runtime status` o `gnx credentials set` hasta que existan.

## Alcance a comprobar

### A. Host Windows / identidad dedicada

Base documentada: `gnx-runtime`, servicio `GNXRuntime`, rutas `C:\Program Files\GNX-0.3.1`, `C:\ProgramData\GNX-0.3.1` y setup separado `C:\ProgramData\GNX-Setup-0.3.1`.

Pruebas:

- `gnx-setup.exe --check` detecta prerequisitos/conflictos sin mutar.
- `gnx-setup.exe --provision` crea/reconcilia cuenta dedicada y servicio.
- ACLs impiden acceso no autorizado a runtime/data.
- `operator.sid` queda registrado sin exponer secretos.
- Reinicio no rompe servicio ni estado.

### B. Wide Linux runtime GNX

Base documentada: distro `GNX-0.3.1`; `GNX` o `gnx-node` son conflicto legacy, no adopción.

Pruebas:

- setup/import usa nombre `GNX-0.3.1`.
- Wide Linux/systemd quedan disponibles según prerequisitos.
- después de reboot, `GNXRuntime` puede verificar/continuar bootstrap.
- acceso operativo normal es vía GNX/broker, no por flujo manual del usuario.

### C. Servicios Linux / quadlet

Pedido 360: validar servicios tipo quadlet para `app.gnx` y `compute.gnx` sobre HTTPS.

Estado actual a doble-check:

- `compute.gnx` sí aparece como contrato normativo.
- `app.gnx` se conserva como superficie propia de presentación HTML/CSS/JS; sus assets/serving son implementación pendiente, no una nueva capacidad.

Pruebas sin inventar arquitectura:

- confirmar unidades/quadlets realmente generadas por el bundle actual;
- confirmar `https://compute.gnx` con TLS válido;
- si `app.gnx` aún no tiene assets, mantenerlo como implementación pendiente y no declararlo operativo;
- verificar reinicio y health sin imprimir secretos.

### D. Credenciales y secretos

Usar sólo interfaces existentes:

- `gnx connect --setup-key-file <file>` para setup key por archivo, no valor en argv.
- `gnx access configure` para prompt humano oculto.
- `gnx credentials control|compute` sólo revela cuentas guardadas según contrato actual.

Criterios:

- nada secreto en argv/logs/evidencia;
- prompts no-echo donde aplique;
- errores sanitizados;
- evidencia con redacción.

### E. Branding / instalador

Assets históricos disponibles en `origin/legacy`:

```text
assets/banner-install-side.png
assets/bg-installer-banner.png
assets/branding-install-logo.ico
assets/branding-install-logo.png
assets/tray-icon.ico
assets/tray-icon.png
```

Validar sin modificar `legacy`:

- si el instalador actual ya consume assets GNX;
- si falta icono tray `g` o app/window `G.ico`, registrar gap;
- textos públicos GNX; atribuciones legales en licencia/SBOM/evidencia.

## Gates del ciclo

- **G0 Base:** repo/toolchain/tests verdes.
- **G1 Setup Windows:** check/provision con cuenta dedicada y ACLs.
- **G2 Wide Linux:** distro `GNX-0.3.1`, systemd/reboot ok.
- **G3 HTTPS:** `compute.gnx` operativo; `app.gnx` tendrá gate separado de presentación cuando existan sus assets.
- **G4 Secretos:** sin secretos en argv/logs/evidencia.
- **G5 Branding:** assets instalador/tray/window verificados o gap.
- **G6 Evidencia:** resultado pass/fail/block con comandos reales.

## Evidencia sugerida

```text
evidence/releases/0.3.1/<candidate-commit>/index.json
```

Campos mínimos: commit, host, comandos reales, duración, resultado, hashes, logs sanitizados y gaps.
