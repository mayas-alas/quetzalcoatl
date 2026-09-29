# Voice dentro del LXC

El nuevo runtime exige **Podman 6+ y Quadlet**, también dentro del LXC.
No se actualiza ni se adopta la instalación anterior.

```text
Tailscale → app.gnx → botones HTTPS: computer.gnx / voice.gnx
voice.gnx → gateway GNX (TLS) → IP Tailscale del LXC:8080
                            → Serve TCP → Voice en 127.0.0.1:8080
```

Los certificados `.gnx` pertenecen a la CA del gateway GNX. Tailscale cifra
el transporte privado; sus certificados automáticos `*.ts.net` no certifican
`voice.gnx`. Conservar la CA existente. Sin Funnel ni puertos publicados.
Permitir clientes autorizados → gateway:443 y sólo gateway → sidecar:8080.
HF usa Internet saliente; «privado» describe el acceso al servicio.

## Archivos

- `config/voice.toml`: imágenes por digest y modelos. Sin secretos.
- `config/gnx.private.example.toml`: rutas privadas; sustituir las IP ilustrativas
  por las observadas. `app.gnx` ya es el portal, no necesita otra ruta.
- `runtime/voice/`: Quadlets **externos**, no incrustados en Rust.
- `packaging/linux/build-voice.sh`: compila y copia un conjunto fijo de archivos;
  genera `payload/manifest.json` con SHA-256. No incluye archivos `.env`.
- `ops/voice`: lee ese payload, verifica su hash y suministra archivos.
  No crea guests, descarga imágenes, inicia servicios ni modifica DNS.

## Build

En Linux, desde el repo:

```sh
cargo test --locked --manifest-path ops/voice/Cargo.toml
sh packaging/linux/build-voice.sh /tmp/gnx-voice-build
```

También acepta un segundo argumento con un binario Linux ya compilado para la
arquitectura del guest. El hash del manifest debe llegar por el canal de release
revisado, **no confiar en un hash recibido junto a un payload desconocido**.
El checksum detecta alteraciones; no es por sí solo una firma de autor.

## Guest e instalación

1. Crear un LXC no privilegiado con systemd/cgroup v2 y nesting. Verificar red,
   almacenamiento y Podman 6+ real; no relajar AppArmor ni usar modo privileged.
2. Crear la cuenta `gnx-voice`, home propio no escribible por otros, contraseña
   bloqueada, shell `nologin`, rangos subuid/subgid libres y `loginctl enable-linger
   gnx-voice`. No reutilizar identidades de otros servicios.
3. Cargar las imágenes verificadas en el almacenamiento **rootless de esa cuenta**.
   Completar sus digests en `voice.toml`; valores vacíos impiden el suministro.
4. Entregar secretos fuera de Git/argv/logs en
   `~gnx-voice/.config/gnx/voice/secrets/{hf-token,api-token,enrollment-key}`.
   Directorio 0700, archivos 0600, propietario `gnx-voice`.
5. Ejecutar como esa cuenta, con su HOME y XDG_RUNTIME_DIR, no como root:

```sh
./gnx-voice-config check voice.toml
./gnx-voice-config render voice.toml payload "$MANIFEST_SHA256" staging
./gnx-voice-config supply voice.toml payload "$MANIFEST_SHA256"
systemctl --user daemon-reload
systemctl --user start gnx-voice.service gnx-voice-access.service
./gnx-voice-config route voice.toml
```

`MANIFEST_SHA256` no es secreto. `supply` instala en `~/.config/containers/systemd`
y `~/.config/gnx/voice`. Rechaza destinos existentes y unidades que oculten los
Quadlets. Un fallo de escritura puede dejar archivos parciales: revisar antes de
reintentar. `check` no sustituye una prueba OCI real.

La ruta resultante usa la IP real del sidecar online. Mezclarla con las rutas del
gateway **nuevo**, sin borrar otras rutas, y usar `gnx doctor/plan/apply/status`.
No ejecutar el instalador encima del runtime anterior. Este archivo describe
el procedimiento; no demuestra que ya esté instalado.

## Pruebas antes de darlo por funcionando

- Generación real de Quadlets con Podman 6+, OCI rootless, secretos legibles sólo
  por el servicio; identidad Tailscale persistente y ACL mínima.
- DNS privado y TLS confiable para app/compute/computer/voice; conservar los aliases.
- Botones de `app.gnx`, rechazo de API sin autenticación, HF texto y audio real.
- Reinicio del guest: unidades e identidad vuelven; ningún puerto público nuevo.
- Probar móvil sólo si se dispone realmente del dispositivo.

Mantener el volumen Tailscale. Tras retirar la clave de alta, conservar su archivo
vacío y protegido para no romper el mount. No escribir secretos en capturas.
