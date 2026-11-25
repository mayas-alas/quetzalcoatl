# Kestra

Desde una consola administrativa del producto instalado:

```powershell
gnx deploy --stack kestra
```

Reutiliza el pipe administrativo y el despliegue Rust a un **LXC nuevo no
privilegiado**: 2 vCPU, 6 GiB RAM (Kestra requiere al menos 4 GiB, más PostgreSQL
y sistema), disco según `config/lxc.json`. No instala ni activa aplicaciones al
instalar/arrancar GnX Core. No adopta el guest kluster ni otro stack registrado.
Las imágenes de aplicaciones se descargan al desplegar, no están en los inputs
offline del núcleo. Requiere conectividad al registro.

Los Quadlets y la plantilla YAML están incorporados con `include_str!` en el
CLI/servicio Rust, por lo que MSI y EXE los incluyen sin descargar scripts al
instalar. `kestra-input.json`, bajo las ACL administrativas de
`%ProgramData%\GnX Core`, conserva el bundle exacto ligado al SID/GUID WSL antes
de reservar un VMID. Repetir deploy, actualizar o reinstalar conserva imágenes,
configuración y credenciales guardadas; no aplica definiciones nuevas sobre un
despliegue existente. Una entrada corrupta o de otra identidad bloquea, sin
regenerar claves. El inventario de recreación reconoce su ownership; la
recreación destructiva sigue necesitando la confirmación propia del producto.

Kestra 2.0.11 y PostgreSQL 16.16-bookworm usan digests OCI oficiales con plataforma
linux/amd64. PostgreSQL queda en la red bridge del guest, sin puerto publicado.
La UI escucha en **127.0.0.1:8080 del guest**: no en el loopback de Windows. No se
añade dominio, proxy público ni portproxy. El acceso remoto necesita un túnel
administrativo autenticado ya disponible hasta ese loopback; este perfil no
crea cuentas SSH ni abre otro mecanismo de acceso. No se publica el puerto de
gestión 8081.

El usuario inicial es `admin@kestra.gnx`. Las contraseñas aleatorias de UI y base
de datos se generan una sola vez y se conservan en la entrada protegida; el
administrador puede consultarlas localmente con una herramienta privada que
respete esas ACL. No copiarlas al chat, argumentos o logs. En el guest,
`/etc/gnx-app/config/kestra.yml` contiene la configuración y
`kestra-postgres.env` la credencial de PostgreSQL. El archivo YAML se monta de
solo lectura, con `:U` para el usuario no root de la imagen Kestra; su dueño en
el guest cambia al UID de ese usuario. El directorio de configuración sigue
privado. No modificar únicamente el guest esperando que eso cambie el bundle
Windows guardado.

Los datos viven bajo `/var/lib/gnx-app/kestra-storage` y
`/var/lib/gnx-app/kestra-postgres`, independientes del contenedor OCI. Systemd
gobierna `kestra.service` y `kestra-postgres.service`; PostgreSQL debe pasar su
healthcheck antes del inicio de Kestra. El timeout de parada de Kestra permite
sus cinco minutos de finalización. Actualizar/reinstalar Windows no borra el
guest ni esos datos. Eliminar el LXC o recrear el nodo sí pierde sus datos;
conservar el bundle no es un respaldo de la base ni del almacenamiento.

No se monta socket Podman/Docker, ni se activa su API. Las tareas de scripts
deben seleccionar explícitamente `io.kestra.plugin.core.runner.Process` para
ejecutarse dentro del contenedor Kestra con las herramientas de esa imagen.
Los flujos que requieran Docker no funcionan con este perfil. No se incorpora
otro runner, supervisor, Redis ni servicio de almacenamiento: PostgreSQL
resuelve cola/repositorio y el almacenamiento es local persistente.

El despliegue comprueba unidades activas mediante la ruta existente; no acredita
login, ejecución de un flujo o recuperación de datos. Esta entrega no aplica
Kestra a Proxmox ni acepta runtime en este host.

Referencias oficiales: [Compose de Kestra 2.0.11](https://github.com/kestra-io/kestra/blob/v2.0.11/docker-compose.yml),
[requisitos](https://kestra.io/docs/administrator-guide/requirements),
[Podman](https://kestra.io/docs/installation/podman-compose),
[Process runner](https://kestra.io/docs/task-runners/types/process-task-runner),
[Quadlet](https://docs.podman.io/en/stable/markdown/podman-systemd.unit.5.html) y
[imagen PostgreSQL](https://hub.docker.com/_/postgres).
