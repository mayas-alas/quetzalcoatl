# GnX Proxmox — OCI candidate and promotion

## Estado

La imagen propia está construida desde Debian y no instala
`pve-fake-subscription`. **Todavía es un candidato, no una actualización del nodo**.
`release.json` tiene `status: candidate` y `registry_repository: null`: falta
identificar el repositorio OCI autorizado. No se inventa un registry ni se
reutiliza automáticamente la credencial GitHub del usuario.

La subida anterior de un archivo OCI a Hugging Face es un artefacto de la
aproximación descartada: no es la fuente del runtime, no está en el Quadlet y
no se descarga desde el nodo. No se ha borrado ni reescrito ese historial.

## Paradigma

1. **Podman build**: `Containerfile` + `prepare.run` + runtime upstream versionado.
2. **Skopeo inspect/copy**: `containers-storage:` → `oci-archive:` para sellar el
   candidato; validación/importación OCI → `containers-storage:` preservando digest.
3. **Promoción explícita**: `oci-archive:` → `docker://<repositorio>:<release>` con
   `--preserve-digests`, TLS y política Skopeo normales. Se verifica después la
   referencia remota `docker://<repositorio>@sha256:…`.
4. **Runtime**: Skopeo copia esa referencia inmutable al `containers-storage:`
   de la identidad del nodo. Quadlet `Image=<repositorio>@sha256:…`, `Pull=never`.
   No hay curl, wget, apt, construcción de Proxmox ni canal HTTP de control.

No se agrega Go: Skopeo ya implementa los transportes de containers/image.
Rust sigue orquestando el payload externo, sin URLs, imágenes o YAML incrustados.

## Construcción

En la WSL del usuario host, con Podman, Skopeo y Python 3.11+:

```sh
sudo bash installer/scripts/build-compute.sh
sudo python3 installer/scripts/promote-compute.py check
```

Salidas normales en `build/compute/`: OCI, SHA-256, inventario de paquetes y digest
de manifest. La compresión durante exportación puede cambiar el manifest: se
promueve el digest del **OCI exportado**, no el image ID ni el digest previo de
containers-storage. Una reconstrucción puede cambiar los paquetes apt: se revisan
inventario, hash y manifest y se actualiza el candidato explícitamente.

Para publicar, configurar el `registry_repository` real y autorizado en
`release.json` y usar `promote-compute.py publish`. Las credenciales provienen del
almacén estándar protegido de containers; nunca de argv, URLs, logs o variables
de entorno. La promoción falla con errores de autenticación o con una etiqueta
existente que apunta a otro digest. Solo produce `build/compute/promoted.json`;
**no cambia el Quadlet ni reinicia el nodo**.

## Gate de integración e impactos

- No se activa `import-image.run` en install/reconcile mientras el candidato no
  esté publicado. El importer rechaza candidatos, tags sin digest, TLS desactivado
  implícito y discrepancias entre configuración, manifest y Quadlet.
- Antes de activar: empaquetar el registro promovido, añadir Skopeo a las
  dependencias del nodo y llamar al importer **antes** de copiar/reiniciar el
  Quadlet, tanto en alta como en Retry. Un fallo de adquisición debe dejar la
  imagen y el compute anterior intactos.
- La identidad Windows/WSL del nodo tiene un almacén distinto al builder. Una
  imagen privada necesita credencial propia y provisioning explícito; no se
  hereda la sesión del host. Preferir lectura anónima del artefacto publicado.
- Cambiar el digest reinicia compute. No modificar los volúmenes
  `/var/lib/gnx-node/compute` ni `compute-config`, CA, secretos o sidecar RustFS.
  Probar KVM, pveproxy, gateway, VMs/LXC y Retry sin recreaciones repetidas.
- El rebuild mantiene los scripts del commit upstream usado por la imagen
  anterior, pero actualiza paquetes apt. Esto no demuestra compatibilidad de
  datos ni autoriza downgrade después de migraciones Proxmox.
- Actualizar imagen y producto requiere una versión nueva de instalador; no se
  reemplazan los binarios ya publicados de v0.1.53 ni se vuelca el OCI en el Space
  de 1 GB. El almacenamiento/distribución OCI pertenece al repositorio de imágenes.

## Fuentes, licencias y validación

Runtime MIT de `dockur/proxmox`:
`e18ddeb8461b09a2c2e8a9d64ada1272c1442109`. Aviso en `UPSTREAM_LICENSE.md`,
incluido en la imagen. Cambios GnX: AGPL-3.0-only; componentes Debian/Proxmox
conservan sus licencias. Base Debian amd64 y keyring Proxmox fijados por hash.
Apt conserva el repositorio oficial HTTP firmado; no se omite verificación TLS
para el endpoint HTTPS con certificado incorrecto.

Build, export/import con Skopeo, digest OCI, linux/amd64, entrypoint/systemd,
sintaxis upstream y ausencia del paquete falso: verificados. Paquetes observados:
`proxmox-ve 9.2.0`, `pve-manager 9.2.21`, `qemu-server 9.2.10`,
`pve-container 6.1.14`. No se afirma boot/upgrade live de este candidato.
