# Evaluación P1 de rootfs para GNX

Status: análisis P1, no READY

Alcance: Ubuntu 24.04 LTS oficial/WSL, Ubuntu Base y Debian slim para la distribución Wide Linux `GNX-0.3.1` de GNX.

## Reglas aplicadas

- No se descargó, generó, borró ni importó ningún rootfs.
- No se inventan procedencias, URLs ni digests; los digests deben venir de un manifest autenticado o de evidencia de host.
- `legacy` no se leyó ni modificó.
- El contrato público sigue siendo GNX: `doctor`, `plan`, `apply`, `status` y JSON/exit semantics.
- Un gate bloqueado o pendiente queda registrado como pendiente; no se convierte en éxito.

## Hechos observados en el repo

### Contrato y aceptación

- `docs/01-product.md` exige un runtime Linux compartido para Access, Control y Compute, con Windows como puente tipado, distro fija `GNX-0.3.1`, automount e interop de Windows deshabilitados, y sin secretos en intent, argv, logs o evidencia.
- `docs/04-windows-runtime.md` define que setup/provision recibe `--rootfs <tar>` y `--rootfs-sha256 <trusted SHA256>`; el hash del rootfs debe coincidir con `rootfs_sha256` del manifest firmado.
- `docs/05-acceptance.md` requiere G0-G6 en host declarado; loopback, contenedor interno o existencia de un archivo no sustituyen la ruta de cliente remoto ni la salud real.
- `docs/06-release.md` requiere identidad inmutable, digest, licencia y source de todo artefacto de runtime; no hay descargas de reemplazos no fijados en install time.

### Implementación y empaquetado

- `packaging/windows/build.ps1` usa por defecto `BuildDistro='Ubuntu-24.04'` para compilar Linux en WSL y exige un `Rootfs` verificado antes de producir un candidate completo.
- `packaging/windows/install.ps1` valida manifest sellado, artefactos y `rootfs_sha256` antes de invocar `gnx-setup.exe --provision`.
- `packaging/windows/runtime.lock.json` está `unsealed` y tiene `rootfs:null`; el repo no contiene una selección de rootfs lista para release.
- `src/adapter/windows/runtime.rs` importa `C:\ProgramData\GNX-0.3.1\rootfs.tar` si la distro no existe, instala `bundle.tar` con `/bin/tar`, escribe `/etc/wsl.conf` con `systemd=true`, `automount=false`, `interop=false`, `appendWindowsPath=false`, exige `/bin/sh`, y luego borra los bootstrap copies.
- El broker Windows ejecuta en WSL: `/usr/bin/timeout 600 /usr/local/bin/gnx <op> --broker`; por tanto el rootfs necesita `timeout` en `/usr/bin` o una decisión de compatibilidad equivalente.
- El adapter Linux exige Linux x86_64/amd64, root, systemd, cgroup v2, `/dev/kvm`, `/dev/fuse`, `/dev/net/tun`, Podman, `curl`, `openssl` y al menos 32 GiB libres en `/var/lib`.
- `packaging/linux/build.sh` y bootstrap requieren `sh`, `tar`, `sha256sum`, instalación root-owned de `/usr/local/bin/gnx` y `/etc/gnx/gnx.toml` con permisos restrictivos.
- `runtime/release.toml` declara plataforma `linux-amd64` y runtime containers con imágenes por digest; `runtime/compute/gnx-compute.container` no tiene imagen verificada y no debe arrancarse como release completo sin decisión de release.

## Requisitos mínimos comparables para P1

El archivo no ejecutable `docs/rootfs-comparison.requirements.json` fija los checks que deberían evaluarse en host o laboratorio sin seleccionar proveedor ni digest. Resumen humano:

| Dimensión | Requisito P1 |
| --- | --- |
| Arquitectura | amd64/x86_64 compatible con WSL2 y el binario Linux GNX. |
| Importación Wide Linux | Importable como distro `GNX-0.3.1` por el owner esperado; no adoptar distros existentes. |
| systemd | `systemd=true` operativo tras reinicio/terminate de WSL. |
| Podman | Disponible y compatible con cgroup v2; no basta instalar binario si no puede correr containers. |
| Shell/base tools | `/bin/sh`, `/bin/tar`, `/usr/bin/timeout`, `sha256sum`, `curl`, `openssl` disponibles. |
| Seguridad WSL | Automount, interop y appendWindowsPath deshabilitables y verificables. |
| Tamaño | Medir tamaño comprimido, tamaño importado y crecimiento tras bootstrap; no aceptar sin evidencia local. |
| Soporte | Debe tener ciclo de soporte declarado por la fuente auténtica y flujo de actualizaciones compatible con release sellada. |
| Superficie de ataque | Minimizar paquetes preinstalados, servicios activos y canales host/guest; no sacrificar gates de runtime. |

## Opciones evaluadas

### 1. Ubuntu 24.04 LTS oficial/WSL

**Hechos del repo**

- Es el único nombre de base ya referenciado por scripts: `Ubuntu-24.04` en `packaging/windows/build.ps1` y `packaging/windows/provision-gnx-runtime.ps1`.
- La documentación actual exige distro GNX propia, no que el usuario opere directamente la distro base.
- El provisionador alternativo existente usa `wsl.exe --install Ubuntu-24.04 --no-launch --web-download`, exporta, calcula hash local, elimina la base e importa bajo `GNX-0.3.1`; ese flujo aún no equivale al manifest firmado exigido por `install.ps1`.

**Supuestos a validar**

- Que la imagen WSL oficial de Ubuntu 24.04 puede exportarse/importarse bajo el runtime account sin primer-login interactivo.
- Que trae o permite habilitar `systemd`, `sh`, `tar`, `timeout`, `curl`, `openssl` y Podman sin descargas no fijadas durante install time.
- Que el tamaño resultante es aceptable para el candidate.

**Ventajas**

- Mejor alineación con el repo actual y con el build distro por defecto.
- Mayor probabilidad de compatibilidad WSL/systemd sin parches propios.
- Menor delta de implementación para P1: se selecciona una ruta ya nombrada, pero se endurece con manifest/digest en vez de confiar en instalación dinámica.

**Riesgos/rechazo**

- Rechazar si la procedencia oficial, licencia/SBOM, digest exacto o soporte no pueden registrarse en manifest sellado.
- Rechazar si requiere descarga mutable en instalación o bootstrap.
- Rechazar si Podman/systemd/cgroup v2 falla en el host declarado.

### 2. Ubuntu Base

**Hechos del repo**

- No aparece como rootfs elegido ni como URL/digest en `runtime.lock.json` o manifest sellado.
- GNX ya instala su propio bundle y configura `/etc/wsl.conf`, por lo que un rootfs mínimo es conceptualmente compatible si cumple los comandos y gates.

**Supuestos a validar**

- Que la variante base concreta para amd64 se importa correctamente en WSL2.
- Que incluye o puede recibir sin mutación no fijada: systemd operativo, Podman, `curl`, `openssl`, `timeout`, `tar`, `sha256sum`.
- Que el ahorro de tamaño compensa el trabajo de hardening y evidencia.

**Ventajas**

- Menor superficie inicial que una distro WSL de propósito general.
- Buena alternativa si se quiere un rootfs sellado, controlado y reducido.

**Riesgos/rechazo**

- Mayor probabilidad de gaps de primera inicialización WSL, systemd o paquetes base.
- Rechazar si necesita una etapa de apt no fijada durante instalación o si el resultado no queda reproducible con digest, licencia y SBOM.

### 3. Debian slim

**Hechos del repo**

- No está referenciado por scripts ni docs como builder o runtime.
- El producto no promete proveedor de distro; Debian podría ser release fact si cumple gates, pero cambiaría supuestos operativos del trabajo actual.

**Supuestos a validar**

- Que el rootfs slim concreto soporta WSL2, systemd y Podman con cgroup v2 en el host declarado.
- Que las rutas esperadas (`/bin/sh`, `/bin/tar`, `/usr/bin/timeout`, `/usr/local/bin/gnx`) existen o se corrigen antes de sellar el artifact.
- Que soporte, licencia y digest se pueden registrar sin inventar procedencia.

**Ventajas**

- Potencialmente menor tamaño y superficie de ataque que una imagen WSL general.
- Base estable si se construye una cadena de release propia.

**Riesgos/rechazo**

- Mayor delta frente al repo actual y mayor riesgo de incompatibilidad WSL/systemd/Podman.
- Rechazar como P1 si requiere adaptar contrato, bootstrap o paths públicos; cualquier cambio así excede esta evaluación.

## Recomendación P1

**Candidate recomendado: Ubuntu 24.04 LTS oficial/WSL, pero sólo como rootfs sellado/importable por GNX, no como descarga dinámica en instalación.** Es la opción con más señales internas del repo (`Ubuntu-24.04` ya se usa para build/provision experimental) y probablemente minimiza el cambio para llegar a evidencia P1. La aceptación requiere que el coordinador aporte procedencia oficial, digest, soporte/licencia y manifest sellado; sin eso no debe etiquetarse como candidate release.

**Alternativa: Ubuntu Base amd64.** Es la alternativa preferida si el objetivo primario es reducir superficie/tamaño y el equipo acepta crear evidencia de bootstrap reproducible. Debian slim queda como opción de investigación posterior, no como alternativa P1 principal, porque el repo actual no la referencia y el delta de compatibilidad es mayor.

## Criterios de aceptación

Aceptar una opción sólo si todos estos puntos tienen evidencia local o de release, no suposición:

1. Manifest sellado registra `rootfs_sha256`, tamaño, identidad de source, licencia/SBOM y plataforma `windows+linux-amd64`.
2. `gnx-setup.exe --check` y `--provision` verifican manifest/rootfs y devuelven `ACTION_REQUIRED`, no `READY`, hasta que bootstrap/health pase.
3. Importación WSL como `GNX-0.3.1` funciona bajo el owner esperado, con automount/interoperabilidad/Windows PATH deshabilitados.
4. Dentro de la distro: `uname -m` es x86_64/amd64, systemd funciona, cgroup v2 está disponible, Podman corre un contenedor local ya fijado o el gate falla honestamente.
5. Existen y funcionan `/bin/sh`, `/bin/tar`, `/usr/bin/timeout`, `curl`, `openssl`, `sha256sum`.
6. `gnx doctor`, `gnx plan`, `gnx apply`, `gnx status` preservan JSON único y semántica de exits; ningún gate faltante se reporta como éxito.
7. Tamaño comprimido/importado y superficie de paquetes/servicios se registran y son aceptados por release.
8. G0-G6 pendientes de host se ejecutan en la matriz declarada antes de cualquier READY.

## Criterios de rechazo

Rechazar cualquier propuesta que:

- dependa de URL, digest o procedencia no documentada en manifest/evidencia;
- descargue paquetes/rootfs durante instalación sin pinning y autenticación de release;
- requiera meter secretos en argv, env, logs o evidence;
- cambie nombres públicos GNX o adopte distros/roots legacy;
- no soporte systemd/Podman/cgroup v2 en WSL2;
- no tenga `/usr/bin/timeout`, `/bin/sh` o `/bin/tar` en las rutas usadas por el código;
- declare `READY` antes de bootstrap, doctor/status y gates de host;
- reduzca superficie rompiendo Access, Control, Compute o recuperación.

## Gates que requieren host

- Verificación de importación WSL por owner `gnx-runtime` y detección de nombres conflictivos.
- `systemd`, cgroup v2, Podman, `/dev/kvm`, `/dev/fuse`, `/dev/net/tun` y almacenamiento real.
- Medición de tamaño comprimido, tamaño importado y espacio libre post-bootstrap.
- Ejecución de `gnx-setup.exe --check`, `--provision`, bootstrap y `doctor/status` en Windows limpio.
- G2/G3 desde cliente autorizado remoto: DNS `.gnx`, private reachability, TLS y aislamiento de puertos.
- Persistencia tras reapply, restart y reboot.

## Propuestas que el coordinador debería rechazar como especulativas

- Cualquier afirmación de que Ubuntu Base o Debian slim son compatibles sin importar y ejecutar los checks anteriores.
- Cualquier URL/digest de rootfs no presente en manifest sellado o evidencia externa revisada.
- Promover `packaging/windows/provision-gnx-runtime.ps1` tal cual como release path: hoy usa instalación WSL por nombre y hash local, pero no sustituye el rootfs autenticado requerido por `install.ps1`.
- Declarar que una imagen menor es más segura sin registrar paquetes, servicios activos, actualizaciones y gates de funcionalidad.
