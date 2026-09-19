# TEMP 360 Tracker — alcance mínimo sin crecer arquitectura

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

### B. WSL runtime GNX

Base documentada: distro `GNX-0.3.1`; `GNX` o `gnx-node` son conflicto legacy, no adopción.

Pruebas:

- setup/import usa nombre `GNX-0.3.1`.
- WSL/systemd quedan disponibles según prerequisitos.
- después de reboot, `GNXRuntime` puede verificar/continuar bootstrap.
- acceso operativo normal es vía GNX/broker, no por flujo manual del usuario.

### C. Servicios Linux / quadlet

Pedido 360: validar servicios tipo quadlet para `app.gnx` y `compute.gnx` sobre HTTPS.

Estado actual a doble-check:

- `compute.gnx` sí aparece como contrato normativo.
- `app.gnx` debe tratarse como **gap o decisión pendiente** si no existe en docs/código.

Pruebas sin inventar arquitectura:

- confirmar unidades/quadlets realmente generadas por el bundle actual;
- confirmar `https://compute.gnx` con TLS válido;
- si `app.gnx` no existe, registrar gap antes de implementar;
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
- **G2 WSL:** distro `GNX-0.3.1`, systemd/reboot ok.
- **G3 HTTPS:** `compute.gnx` ok; `app.gnx` gap o prueba real si existe.
- **G4 Secretos:** sin secretos en argv/logs/evidencia.
- **G5 Branding:** assets instalador/tray/window verificados o gap.
- **G6 Evidencia:** resultado pass/fail/block con comandos reales.

## Evidencia sugerida

```text
evidence/releases/0.3.1/<candidate-commit>/index.json
```

Campos mínimos: commit, host, comandos reales, duración, resultado, hashes, logs sanitizados y gaps.
