# GnX Core 0.0.14

Candidata para Windows x64: nodo privado con Tailscale, Pi-hole, Caddy y Proxmox VE
en una WSL dedicada, operado mediante una CLI y servicios nativos.

## Elegir una descarga

| Archivo | Finalidad |
| --- | --- |
| **GnxCoreSetup.exe — recomendado** | Instalación interactiva. Burn contiene y ejecuta el MSI del producto. |
| GnxCore.msi | Instalación directa con Windows Installer para administración o despliegue gestionado. |
| SHA256SUMS.txt | Hashes de los dos instaladores para comprobar integridad. |

**Instala solo una de las dos vías.** Ambas contienen el mismo núcleo offline y
tienen tamaños similares por ese motivo. No son componentes complementarios.
La CLI se instala con el producto; no requiere una descarga separada.

## Requisitos y uso

- Windows x64 con WSL2 y virtualización disponibles; permisos administrativos.
- Los instaladores no incorporan los prerrequisitos Windows que falten.
- Tras la preparación, acceso en `https://app.gnx` y `https://compute.gnx`.
- Tailscale y la contraseña de Proxmox se configuran desde la CLI instalada,
  mediante entrada sin eco. La app no solicita secretos.
- Usa la entrada de Windows de la vía elegida para mantenimiento/desinstalación;
  no ejecutes el otro formato para completar o reparar la instalación.
- Si ya instalaste otra compilación 0.0.14, desinstálala antes de probar esta
  candidata de la misma versión. La retirada normal conserva cuenta, perfil,
  WSL, CA, credenciales y datos; no es una purga.

## Cambios del núcleo

- Recuperación de acceso mesh mediante `tailscale wait` en el Quadlet existente,
  sin bloquear la app local mientras la mesh espera enrolamiento.
- Reconciliación de DNS y escucha del gateway con bloqueo y comparación antes
  de reemplazar configuración o solicitar reinicios.
- Mantenimiento HTTPS nativo, conservando CA y claves. Sin comando TLS ni acción
  de importación de certificados en la app normal.
- Retirada MSI coordinada antes de quitar el servicio Windows y reinstalación
  conservadora, sin borrar identidad ni datos.

## Estado de la candidata

Instaladores sin firma Authenticode. SHA-256 permite comprobar integridad, no
sustituye una firma de código. GitHub requiere una sesión con acceso al repositorio
privado. Los enlaces automáticos «Source code» son fuentes, no instaladores.

El empaquetado CI, la instalación MSI y el mantenimiento Burn son comprobaciones
distintas. No se acredita aceptación general en Windows limpio, recuperación tras
reboot, expiración futura de certificados ni acceso desde otros dispositivos.
El despliegue de archivos/Quadlets hacia LXC todavía no está implementado.
