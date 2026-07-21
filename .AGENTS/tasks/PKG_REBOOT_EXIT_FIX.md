# PKG-02 — Burn debe reconocer REBOOT_PENDING

Estado: EN PROGRESO

## Alcance

- Mapear el exit code 14 de `gnx-host-preflight prepare-wsl` a `scheduleReboot`.
- Mantener sincronizado el valor Rust con el bundle mediante un guard de build.
- Reconstruir el instalador y repetir la instalación real en Windows Dockur limpio.

## Criterio de cierre

- Burn ya no presenta `0x8007000e` cuando WSL requiere reinicio.
- El instalador termina o programa un reinicio real y puede continuar/reanudarse.
- Evidencia noVNC y artefacto de Actions conservados sin afirmar G5.
