# Revisión independiente PKG-02: contrato de reinicio Burn

Actúa únicamente como revisor. No edites archivos, no hagas commit ni push.

Revisa el diff no confirmado de `installer/bundle.wxs`, `installer/build.ps1` y `docs/VALIDATION.md` junto con el flujo real de `crates/host-preflight`. Reporta hallazgos P1/P2/P3 con ruta y línea; si no hay defecto accionable, responde exactamente `CLEAN` y añade verificación breve.

Hecho observado: en Windows Dockur limpio, `PrepareWsl` habilitó características y devolvió `REBOOT_PENDING=14`; Burn lo trató como error `0x8007000e`. El diff mapea 14 a `scheduleReboot` únicamente en `PrepareWsl` y añade un guard de build.

Comprueba de forma explícita:

1. Qué hace Burn después de `scheduleReboot`: recorre mentalmente **toda** la cadena. Determina si el posterior `ValidateHost` se ejecuta antes del reinicio y si su `gnx-host-preflight --format json` volverá a devolver 14 por `pending_reboot`. Comprueba sus mappings de `ExitCode`; un error repetido más adelante es P1.
2. Que una solución correcta permita instalar/reanudar de forma determinista sin convertir fallos genuinos (10–13, 15–16, 20, 64) en éxito.
3. Que el guard lea el valor Rust real, se limite al/los paquetes que deben aceptar reinicio, detecte ausencia/orden incorrecto frente al catch-all y no produzca falsos positivos por regex codiciosa entre paquetes.
4. Que WiX acepte la sintaxis y que el comportamiento preserve 3010.
5. Que la documentación no afirme que Dockur cierra G5.

Usa validaciones de solo lectura si ayudan. No amplíes el producto ni propongas casos fuera del PoC/MVP.
