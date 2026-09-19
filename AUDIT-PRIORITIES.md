# GNX — auditoría priorizada y plan de entrega

Fecha: 2026-09-19

## P0 — bloquear entrega hasta resolver

- [x] Eliminar descarga/exportación dinámica de Ubuntu durante instalación.
- [x] Rootfs cubierto por SHA256 en metadata y manifest.
- [x] Manifest sellado y firma detached presente.
- [x] Verificar que los hashes del manifest coinciden con los seis artefactos.
- [ ] Ejecutar instalación real en Windows elevado con el rootfs exacto.
- [ ] Verificar reboot, servicio `GNXRuntime`, WSL `GNX-0.3.1`, `doctor` y `status`.
- [ ] Confirmar persistencia de identidad/storage y rollback en host desechable.

**Estado P0:** candidato técnicamente empaquetado; runtime host aún no aceptado.

## P1 — necesario para entrega controlada

- [x] Tests Rust bloqueados: actualmente pasan; queda un test Linux root ignorado.
- [x] `gnx.exe doctor` produce JSON válido y exit 2 cuando no existe broker.
- [x] Provisionador legacy falla cerrado y ya no se distribuye desde `install-host.ps1`.
- [ ] Incluir un launcher/documentación de instalación en `dist` para evitar entregar EXEs sueltos.
- [ ] Registrar hash del rootfs junto al paquete entregado.
- [ ] Entregar `dist` y `rootfs.tar` como una pareja inseparable.

## P2 — importante después de la demo

- Reproducibilidad del rootfs Ubuntu custom.
- Pruebas G0-G6 completas.
- Validación remota de DNS, TLS, Access y Compute.
- Rebuild desde checkout limpio y promoción con evidencia sanitizada.
- Actualizaciones y recuperación tras reboot.

## P3 — optimización/no bloqueante

- Reducir tamaño del rootfs.
- Eliminar documentación/locales innecesarios.
- Convertir el rootfs en imagen mínima especializada.
- Automatizar publicación y generación de notas de release.

## Decisión de entrega ahora

Se puede entregar como **Windows candidate / controlled preview**. No debe
anunciarse como producción READY hasta completar P0 de host. El primer trabajo
activo es hacer que `dist` sea autoexplicativo y ejecutable mediante el wrapper
autenticado; no se modifica el rootfs ni se introduce otra distro.
