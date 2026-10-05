# GnX Node v0.1.53

## Changed

- GitHub en `app.gnx`: una sola configuración con token clásico, comprobación de permisos `repo`/`workflow`, consentimiento, creación de repositorio privado e instalación del YAML seleccionado.
- Sin protocolo nativo, UAC, OAuth, importación de `gh` ni controles iniciar/detener para GitHub en la PWA. La CLI nativa anterior permanece por compatibilidad.
- El token se mantiene solo durante la operación, sin persistencia. La API GitHub es el único origen externo permitido en CSP; no se siguen redirecciones ni se envían cookies.
- No se sobrescriben repositorios existentes, no se ejecuta Actions automáticamente y los secrets requeridos se configuran en GitHub.
- `releases/` contiene únicamente esta versión. Las notas históricas están en `docs/releases/`, indexadas en `CHANGELOG.md`.

## License

GNU Affero General Public License v3.0 únicamente (`AGPL-3.0-only`). El workspace y sus tres paquetes declaran la licencia; Hugging Face publica `agpl-3.0`. El MSI incluye `LICENSE` y Burn enlaza el texto completo. Dependencias e imágenes de terceros conservan sus licencias originales.

## Validation — 2026-10-05

- `cargo test --workspace --locked`: 62 pruebas aprobadas.
- Build canónico Rust release, MSI y Burn: aprobado, sin errores WiX.
- `installer/scripts/verify.ps1`: rootfs sellado, tamaños y hashes MSI/Burn verificados.
- Contratos estáticos de instalador, runtime, LXC, red/PWA y workload: aprobados.
- Pruebas GitHub en memoria: creación privada e inyección YAML; denegación de permisos, origen no autorizado, consentimiento/validación y errores parciales.
- Host existente: `app.gnx` y `compute.gnx` responden HTTP 200 con validación de cadena/nombre TLS; Schannel requiere omitir la consulta de revocación para la CA privada (`--ssl-no-revoke`, no `--insecure`). Este chequeo corresponde al producto ya instalado, no prueba el despliegue de v0.1.53.

No se afirma instalación/upgrade live de esta versión, creación real de un repositorio GitHub con token de usuario, ejecución de Actions, aceptación desde teléfono ni instalación en host limpio. Los comandos de inspección sin fixtures están en `tests/security/network-pwa-validation.md`.

## Downloads

Descarga `GnxNodeSetup.exe` y verifica contra `SHA256SUMS.txt`.
El bundle incluye el MSI exacto construido y verificado; no se publica una copia MSI separada para respetar la cuota de 1 GB de Hugging Face sin purgar el historial. Ambos outputs se producen con el build canónico; publicación: `installer/scripts/verify.ps1 -PublishRelease -BundleOnly`. La versión 0.1.53 actualiza 0.1.52 sin cambiar los UpgradeCodes; conserva identidad, distro, CA y datos según el contrato de upgrade.
