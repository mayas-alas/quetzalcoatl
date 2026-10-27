# GnX Core 0.0.16

## Descargas para Windows x64

| Archivo | Uso |
| --- | --- |
| **GnxCoreSetup.exe — recomendado** | Entrada interactiva Burn; contiene y ejecuta el MSI. |
| GnxCore.msi | Instalación administrativa directa mediante Windows Installer. |
| SHA256SUMS.txt | Verificar los dos instaladores; no es otro componente. |

Elegir una sola vía. Ambos contienen el mismo núcleo offline; no son pasos
complementarios. El MSI gobierna la transacción Windows, el servicio prepara el
runtime y systemd/Quadlets gobiernan los contenedores. La CLI se instala con el
producto y no se ofrece como portable.

## Envío de Quadlets

- Envía una carpeta admitida con Quadlets y configuración mediante `gnx deploy`.
- Rust reserva y crea un LXC no privilegiado con `pct`, sin ejecutar scripts
  aportados por el usuario en Windows o en el nodo.
- Prepara los directorios de datos antes de arrancar los Quadlets. Conserva el
  recibo y el guest parcial ante fallos; no duplica LXC al reanudar la misma entrada.
- Mantiene MSI como dueño de la instalación Windows y EXE como entrada interactiva.

Los servicios los gobierna systemd dentro del LXC. La creación y el estado activo
de sus unidades no acreditan salud, acceso externo, login ni integración OIDC.
No incluye todavía subida web, builds, nuevas rutas del gateway ni peers mesh de
aplicaciones. Las imágenes de aplicaciones se obtienen por digest desde sus registros;
el despliegue requiere conectividad. Los requisitos Windows/WSL2 deben estar
disponibles: el EXE no instala prerrequisitos adicionales.

La instalación conserva cuenta, WSL, CA, credenciales y datos; no hay purga.
Instaladores sin firma Authenticode. El empaquetado, la instalación, el mantenimiento
Burn y la aceptación de aplicaciones son comprobaciones distintas. No se garantiza
aceptación general en un host limpio, recuperación tras reboot ni login humano.

Se verificaron creación y transferencia por el pipe, servicios en el guest,
consulta idempotente sin duplicación y conservación tras parada/arranque del LXC.
La comprobación HTTP interna de las aplicaciones no acredita acceso remoto ni OIDC.
