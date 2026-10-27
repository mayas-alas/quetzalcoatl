# GnX Core 0.0.15

## En preparación

No publicada: la comprobación real creó y transfirió el LXC correctamente,
pero encontró que no se preparaban los directorios de datos de bind mounts.
Corrección en 0.0.16; no usar esta candidata como aceptación del despliegue.

- Admisión acotada de carpeta con Quadlets y configuración por `gnx deploy`.
- Transferencia por el pipe administrativo existente y creación de LXC nuevo con
  `pct` desde Rust, sin scripts aportados por el usuario en el nodo.
- Los servicios pertenecen al guest y los gobiernan systemd/Quadlets.

No publicar esta candidata hasta comprobar despliegue real e instaladores.
El envío web y las integraciones/login de las aplicaciones no quedan acreditados
por generar un LXC ni por observar sus unidades activas.
