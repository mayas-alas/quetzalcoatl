# GnX Core v0.0.8

- `status` consulta el target systemd antes de mostrar su última fase; una WSL
  aún abierta no se presenta como un nodo activo ni despierta una distro detenida.
- Distingue la fase publicada de la salud actual de HTTPS, mesh y compute.
- Permite recuperar una sesión WSL cuyo cliente propio ya terminó, sin crear
  políticas nuevas de reinicio de contenedores.

La actualización conserva CA, credenciales, identidad mesh y datos. La purga de
la distro no forma parte de `stop` ni de una actualización.
