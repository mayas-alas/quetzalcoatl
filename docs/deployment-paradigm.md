# Paradigma de despliegue GNX

## Decisión vigente

El runtime soportado para este host es:

```text
Windows host
  -> GNXRuntime bajo .\gnx-runtime
  -> distro WSL aislada GNX
  -> binario Linux GNX inyectado y verificado desde el EXE/instalador
  -> Quadlet/systemd dentro de GNX
  -> Access, Control y Compute
```

La distro Ubuntu existente es una instalación heredada. Puede servir para
extraer evidencia o migrar estado, pero no es una segunda implementación ni la
autoridad del producto.

LXC está fuera del roadmap actual. No se agrega una capa de provisión LXC.
Proxmox solo permanece como implementación de Compute si el release definido lo
requiere; GNX debe conservar un único dueño de su lifecycle y de sus probes.

## Limpieza controlada del host

Antes de eliminar estado, registrar únicamente hashes, versiones, nombres de
servicio, propietarios y revisiones; nunca copiar contraseñas, claves, cookies,
tokens ni discos completos al reporte.

La secuencia es:

1. detener y verificar el servicio heredado que no pertenezca a `GNXRuntime`;
2. identificar los directorios heredados de `C:\ProgramData\GNX` y sus
   propietarios;
3. preservar solo la evidencia sanitizada y los datos que se hayan declarado
   migrables;
4. retirar el estado heredado de forma explícita y verificable;
5. instalar/importar la distro WSL `GNX` bajo el usuario dedicado;
6. inyectar el bundle Linux desde el servicio, verificar digest y ownership, y
   borrar el bootstrap temporal;
7. verificar Quadlet, systemd, interop/automount desactivados, broker y probes;
8. ejecutar reapply, restart y reboot antes de declarar la migración válida.

No se debe borrar el estado del host hasta tener identificados los objetivos
exactos y una evidencia de recuperación. `uninstall` debe ser una operación del
producto, con confirmación explícita, ownership checks y un resultado JSON.
