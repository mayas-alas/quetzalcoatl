# Revisión final independiente PKG-02

Actúa solo como revisor. No edites archivos, no hagas commit ni push.

Revisa el diff no confirmado de `installer/bundle.wxs`, `installer/build.ps1`, `docs/VALIDATION.md` y el flujo relevante de `crates/host-preflight`. Devuelve hallazgos P1/P2/P3 con ruta/línea; si no hay defecto accionable, responde exactamente `CLEAN` y una verificación breve.

Comprueba:

1. `REBOOT_PENDING` se lee desde Rust y solo el valor real se acepta.
2. Exactamente `PrepareWsl` y `ValidateHost` mapean 14 una vez a `scheduleReboot`, antes del catch-all; ningún tercer `ExePackage` puede hacerlo.
3. `PrepareWsl` conserva 3010; códigos 10–13, 15–16, 20 y 64 siguen cayendo en error.
4. El guard XML no atraviesa paquetes, detecta duplicados/ausencias/orden y sus parámetros de test no alteran el build normal ni permiten saltarse validaciones en producción.
5. WiX y la semántica completa de cadena/reanudación son coherentes; no reaparece `0x8007000e` en `ValidateHost`.
6. La documentación mantiene Dockur separado de G5.

Puedes ejecutar validaciones read-only del guard y diff. No amplíes alcance.
