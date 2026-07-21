# Revisión independiente CI-03: cierre por evidencia Dockur

Actúa únicamente como revisor. No edites archivos, no hagas commit y no hagas push.

Revisa el diff no confirmado del worktree actual (`codex/ci-dockur`) para determinar si el workflow de GitHub Actions puede terminar en verde **solo** cuando el guest Windows exporta evidencia GNX válida. Reporta hallazgos priorizados P1/P2/P3 con ruta y línea; si no hay defectos accionables, responde exactamente `CLEAN` y añade una verificación breve.

Alcance estricto:

- `.github/workflows/windows-rdp-tailscale.yml`
- `.github/scripts/validate-gnx-evidence.py`
- `README.md`

Comprueba de forma explícita:

1. Que cualquier script invocado exista realmente en runtime. El workflow actual no debe darse por válido suponiendo implícitamente un checkout; verifica si hay `actions/checkout` o si el validador se materializa de otra manera.
2. Que no pueda haber un verde falso por timeout, guest detenido, archivo parcial, hash equivocado, servicio/binario ausente o JSON de estado inválido.
3. Que UTF-8 con BOM y escrituras transitorias/parciales se manejen de forma segura.
4. Que `if: always()` preserve artefactos y cleanup aun cuando la sesión falle.
5. Que ningún secreto ni contenido sensible de la evidencia se imprima en logs, summaries o errores.
6. Que se mantenga el alcance PoC/MVP: esta lane prueba compatibilidad instalable Dockur/noVNC y no afirma G5, quorum físico ni convergencia del cluster sin credenciales.

Puedes ejecutar validaciones de solo lectura. No amplíes el producto ni propongas escenarios fuera de este alcance.
