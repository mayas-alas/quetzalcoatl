# Auditoría del paradigma, builder WSL y sustitución del laboratorio

Fecha: 2026-10-07. Base Git `578e66e`; working tree previo con cambios sin commit
v0.1.78. Esta evidencia corresponde a documentación, inspección y mantenimiento
del builder; **no a una nueva release de la aplicación ni aceptación AI**.

## Investigación previa a la retirada

Se identificaron dos propietarios distintos por registro Windows/SID y ruta:

| Distro | Dueño | Ruta | VHDX observado |
| --- | --- | --- | ---: |
| `GnX-SlimFit` | usuario host `mayas` | `C:\Users\mayas\WSL\GnX-SlimFit\ext4.vhdx` | 31,486,640,128 bytes (~29.32 GiB) |
| `gnx-node` | cuenta dedicada `gnxnodesvc` | `C:\Users\gnxnodesvc\AppData\Local\GnX\wsl\gnx-node\ext4.vhdx` | 31,407,996,928 bytes (~29.25 GiB) |

La lista WSL del host sólo muestra sus distros, no la de `gnxnodesvc`. Se consultó
además `HKEY_USERS/<SID dedicado>/.../Lxss`. No confundir ni borrar ambas.
Servicio `GnXHostAgent` observado Running. CLI instalada: `gnx 0.1.78`; `gnx status`
reportó `[ready] NodeReady`, credenciales conservadas y acceso HTTPS local.

SlimFit estaba detenida; se inició sólo esa distro para inspección. Ubuntu 24.04.3,
Podman/Quadlet 4.9.3; filesystem raíz ~29 GiB usados. Atribución aproximada (`du -x`):

| Consumidor | Tamaño observado | Hallazgo |
| --- | ---: | --- |
| `~/.local/share/gnx-build/ai-import` | 16 GiB | Almacén Podman aislado de imports; sin contenedores ni volúmenes |
| `/var/lib/containers/storage` | 4.8 GiB | 56 imágenes; un `test-nbserver` Created, ninguno activo, sin volúmenes |
| `~/.local/share/containers` | 1.1 GiB | Otro almacén rootless |
| `/tmp/continuity-build-linux` | 1.4 GiB | Compilación del componente fuera del workspace administrado |
| `/var/tmp/container_images_*` | 866 MiB | Imports/exports no retirados |
| `~/build/quetzalcoatl-local` | 250 MiB | Otro target de compilación |
| `~/.rustup` / `~/.cargo` | 584 / 170 MiB | Toolchain/cachés |
| `/tmp/netbird-src`, `/tmp/pocket-src` | 48 / 20 MiB | Clones scratch limpios |

Los tamaños pueden solaparse; no sumarlos como espacio recuperado exacto. Podman
`system df` no cuenta correctamente todos los almacenes aislados ni el VHDX.
No se ejecutó un prune sobre el runtime productivo.

Fuentes Linux únicas revisadas antes de destruir el builder:

- NetBird `/tmp/netbird-src`, commit `eb5a98c`: status limpio.
- Pocket `/tmp/pocket-src`, commit `69065e7`: status limpio.
- `gnx-build/mc-source`, commit `7394ce0`: único untracked `gnx-self.go`.
  Comparación byte a byte confirmó que coincide con
  `installer/tools/mc-self.go` en el repo Windows; el binario ya existe en `build/ai/mc`.
- No repos adicionales bajo los paths de trabajo examinados. No se exportó un
  VHDX de 29 GiB para preservar basura reproducible.
- Se encontró un residuo one-shot de onboarding en `/tmp/gnx-secret-*`, modo 644,
  3 bytes. Se registró sólo metadata, **no se abrió ni se copió su contenido**.
  La retirada de esta distro elimina el residuo; no extrapolar de ello una fuga
  de secretos productivos ni eliminar el contrato protegido actual de IPC.

Espacio C: inicial: 8,054,587,392 bytes libres (~7.50 GiB). Inventario del workspace
Windows (bytes lógicos, no uso físico deduplicado por hardlinks): `build/` 655
archivos / 2,479,889,758 bytes; `target/` 1,567 bytes; input AI 783,262,454 bytes;
input servicios privados 417,877,365 bytes; auth y host ~37.8 MB cada uno.
No se borró `build/last-complete-v0.1.72`, el candidato, inputs sellados ni fuente dirty.

## Laboratorio sustituto comprobado antes de retirar SlimFit

Instalación oficial, sin descargar/exportar un rootfs scratch propio:

```text
wsl --install Debian --name GnX-LinuxLab
    --location C:\Users\mayas\WSL\GnX-LinuxLab --no-launch --web-download
```

Debian 13 (trixie), systemd activo, usuario host Linux `mayas` UID 1000, Podman
5.4.2, Quadlet, jq 1.7, curl, nftables, OpenSSL, Git 2.47.3, Python 3.13.5 y pipx.
Cgroups v2/systemd. No Tailscale identity ni servicios GnX productivos en el lab.
Rootfs usado tras herramientas: ~794 MiB; VHDX 1,110,441,984 bytes al inventariar.

- Migrados por stream tar directo entre distros, sin archivo intermedio:
  token/stored_tokens HF y config/CA de `.mc`. Comparación byte a byte aprobada;
  owner `mayas`, archivos sensibles modo 600 y directorios modo 700.
- No se migraron cachés HF/xet, capas OCI, build targets, clones scratch o estados
  Tailscale/Podman. Ningún secreto fue impreso ni agregado al repo/evidencia.
- Publisher `huggingface_hub==2.0.0`, misma versión que el builder anterior, en
  venv pipx del usuario. `HfApi().whoami()` confirmó autenticación; sólo se emitió
  un booleano, no token ni datos del perfil. PATH del usuario actualizado con pipx.
- Config mc inspeccionada sólo en campos no secretos: aliases default gcs/local/
  play/s3. **No hay destino GnX S3 de publicación configurado en esos aliases**.
  Esto no significa que el S3 productivo no exista: falta un contrato de publicación.
- `bash -n` sobre `compute/lxc.run` y AI `install.run`/`enroll.run`: pasa.
- Generador Quadlet `--dryrun`, con `QUADLET_UNIT_DIRS` apuntando al perfil AI del
  repo: carga network + seis containers, salida 0. No inicia servicios ni acredita AI.
- Podman del lab: cero contenedores y cero volúmenes.

Los comandos de inspección/configuración se ejecutaron directamente por shell/
stdin; no se escribieron nuevos runners o side scripts en el lab ni en Temp.

## Browser skill: observación limitada

`orca computer` (guía del ejecutable de la sesión), Firefox, ventana `1901950`:

- `https://app.gnx/` muestra "Nodo operativo", 100%, checks Windows/WSL/Podman/red/
  gateway/compute correctos; LXC compartido encendido; RustFS, Pocket y NetBird
  mostrados operativos. Esto es proyección UI, no aceptación AI.
- Tab RustFS existente: bucket `continuity`, Private, 16 objetos, 11.2 KiB.
  No se creó bucket ni se subió/borró un objeto. No prueba publicación de artefactos.
- Navegación semántica a `https://compute.gnx/` muestra la aplicación Proxmox y
  diálogo "Proxmox VE Login". No se introdujeron credenciales ni se simuló login.
  No se obtuvo inspección autenticada de los guests desde ese browser.
- El intento de Return por teclado falló por foco, también tras restore; no se
  insistió. Click semántico a la sugerencia de dirección sí cambió la página y el
  estado siguiente corroboró Proxmox/Login. Sin screenshots que incluyan credenciales.

## Estado de la retirada y cierre

**Retirada completada:** se comprobó nombre/ruta/registro del dueño host, se
terminó sólo `GnX-SlimFit` y se ejecutó `wsl --unregister GnX-SlimFit`, salida 0.
El VHDX antiguo ya no existe y su registro desapareció. La única distro registrada
para el usuario host es `GnX-LinuxLab`, usuario predeterminado `mayas`. Tras las
pruebas se terminó sólo el lab para liberar RAM; permanece instalado.

C: final **38,231,662,592 bytes libres (~35.61 GiB)** frente a 8,054,587,392 al
inicio: mejora neta **30,177,075,200 bytes (~28.10 GiB)** después de instalar el lab.
El VHDX dedicado conserva su tamaño observado; `GnXHostAgent` sigue Running.
`gnx status` final volvió a responder salida 0/NodeReady; el diagnóstico confirma
el guest compartido `rustfs` running y con su tag de propiedad. No se desinstaló
GnX ni se destruyeron la distro dedicada, LXC, journals, CA o secretos productivos.

La CLI `gnx diagnose compute` se invocó instalada, sin runner alternativo. La espera
externa de 45 s expiró, pero el agente terminó y escribió el informe canónico en
`C:\ProgramData\GnX\Node\state\compute-diagnostic.txt`. Un `status` durante ese
periodo devolvió servicio no disponible pese a SCM Running; un único recheck
posterior recuperó salida 0. No se reinició/parchó el servicio para esconderlo.

**Hallazgo adicional:** diagnóstico guardado no significa todas las sondas verdes:
`access-transport` exit 124, `access-protocols` exit 1 y `stun-unit` exit 3; release,
container, Proxmox, storage, backend services/discovery y diagnóstico rustfs exit 0.
Continuity business estaba Up. NodeReady es una proyección preservada, no una
aceptación nueva del transporte/STUN ni de AI. Resolver/reportar estos gates por
fuente/build/instalación; no omitirlos porque `diagnose` devolvió un informe.

Se eliminaron también los dos diffs scratch de la primera auditoría creados por
esta sesión en Windows Temp; no se dejaron nuevos scripts de trabajo. Outputs
Windows de la release anterior/candidato e inputs offline se conservaron de forma
deliberada, no se aplicó una limpieza indiscriminada al trabajo heredado.

Validación documental: nueve Markdown con enlaces locales comprobados, ninguno
faltante; `git diff --check` limpio. Suite estática, 13 contratos AI y prueba curl
real/local pasan. No equivalen a build/instalación ni aceptación de los servicios.

## Registro de fuente y publicación

Documentación, reglas e inventario guardados en commit local `81adae7` (nueve
archivos); no se incluyeron los cambios de producto heredados sin commit.
El push al remote fuente HF falló: `Storage limit reached for this space (Max: 1 GB)`
durante la fase LFS. **No afirmar que estos documentos estén publicados.** No se
purga historial, no se salta la verificación LFS y no se publica el candidato AI
para sortear esa cuota. Resolver la publicación de documentación sin duplicar
inputs históricos y mantener el dataset/S3 aprobado para artefactos grandes.

## Límites del trabajo

No se construyó/instaló/publicó una nueva aplicación en esta tarea de documentación
y laboratorio. La clave reusable, motor genérico, upload frontend, logs genéricos,
cleanup automático y publicación S3 son objetivos pendientes de implementación.
El remote fuente actual es HF; no existe un remote GitHub configurado en este repo.
Las reglas Luna 5.6/high quedaron registradas para workers; esta sesión no cambió
su modelo por editar los documentos.
