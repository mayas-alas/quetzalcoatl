# GnX Core 0.0.12

## Cambios

- Retira el comando manual de certificados y su ruta IPC/Rust.
- Mantiene CA y claves; systemd renueva CRL y hoja servidor con antelación de 30 días.
  Publicación atómica, montajes de directorio y aplicación mediante servicios nativos.
- Prepara las direcciones privadas del gateway en su arranque y evita reimportar
  confianza/reconfigurar forwards Windows que ya coinciden.
- Aplica el paquete verificado al instalar/actualizar o reinstalar, no en cada start.
- MSI retira el nodo/acceso Windows propios antes de quitar el servicio; conserva
  cuenta, perfil, WSL, identidad, CA, credenciales y datos para reinstalación.
- Salida status breve, diagnóstico en journal/Visor de eventos y recuperación Rust
  que no trata una tarea interrumpida como recuperación correcta.

## Límites

- Desinstalación no aceptada: puede rechazar una parada ya alcanzada. No usar esta
  versión como referencia de retirada correcta; corrección en 0.0.13.

- Candidato; build, instalación y aceptación son resultados distintos.
- No cambia MTU, red mesh, CA raíz ni protecciones del host. No añade OCSP ni daemon.
- No incluye purga ni despliegue Quadlet/contexto a LXC; esa capacidad sigue pendiente.
- La comprobación manual autorizada en el host no acredita instalación en Windows
  limpio, recuperación tras reboot, vencimiento futuro ni aceptación móvil remota.
