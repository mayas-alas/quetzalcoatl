# GnX Core v0.0.6

- Corrige el paso de argumentos a WSL sin entrecomillar sus opciones.
- Conserva la CLI administrativa, el usuario dedicado y el lifecycle nativo.
- Instala Quadlets con permisos deterministas, sin eliminar unidades ajenas.
- Retira instrumentación temporal y adaptación manual del registro WSL.
- Admite secretos por stdin heredado, además de la entrada interactiva sin eco.
- Mantiene una sesión WSL propia mientras el nodo está iniciado; systemd conserva
  el control de los contenedores y `stop` libera la sesión sin apagar otras WSL.
- Corrige la instalación TLS de Proxmox respetando los permisos de pmxcfs.

La disponibilidad HTTPS, el enrolamiento y la instalación se informan por
separado; compilar el paquete no acredita esos resultados.
