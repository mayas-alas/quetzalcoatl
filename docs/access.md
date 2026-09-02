# Acceso privado desde Android

**Corte:** un nodo de acceso en WSL; DNS privado y los dos HTTPS existentes.
No sustituye el cliente Windows ni el control plane local.

Android usa Tailscale SaaS para alcanzar `gnx-access`; Pi-hole responde
`mesh.gnx` y `*.mesh.gnx` con la IP de ese nodo. Caddy sigue terminando TLS y
enviando cómputo a `8006` interno. DNS no crea servicios ni certificados wildcard.
La VPN, la resolución y la confianza TLS son tres requisitos distintos.

## Aplicar

```powershell
cargo build --release --locked --manifest-path ops/access/Cargo.toml
powershell.exe -NoProfile -File ops/access/prepare-host.ps1
```

El preparador protege `%LOCALAPPDATA%\GNX\access` para usuario, SYSTEM y
administradores. Guardar allí `enrollment.key`: una **Auth key** de Tailscale,
one-off, para un nodo no efímero; no una API key. Repetir el preparador.
Nunca pegarla en chat, argumentos o configuración. El original permanece
protegido en Windows; la copia temporal root-only en WSL se elimina al terminar
el intento. No se necesita otro cliente Windows ni nuevo UAC.

El operador Rust acepta sólo este contrato:

```text
gnx-access apply --config runtime/access/access.toml [--key-file <archivo-protegido>]
```

Valida el corte, arranca Quadlet, enrola por archivo, conserva ID/IP y genera
DNS con la IP observada. Las imágenes están fijadas por digest. No acepta DNS
ni rutas del SaaS, no anuncia subredes, no activa SSH ni exit node. Reaplicar
no vuelve a enrolar un nodo conectado; un cambio de identidad falla cerrado.
`READY access-local` exige DNS UDP/TCP y ambos HTTPS con CA/nombre válidos;
**no acredita acceso remoto**. La salida externa de autenticación no se publica.

## Cerrar la conexión del teléfono

1. En el mismo tailnet, autorizar sólo al Android/usuario previsto hacia la IP
   de `gnx-access`: UDP/TCP `53`, TCP `443` y TCP `80` para la CRL existente.
   Revisar también permisos amplios preexistentes: añadir una regla estrecha
   no revoca un `allow-all`. La Auth key no configura estas políticas.
2. En DNS del SaaS, añadir esa IP como nameserver **restringido a `mesh.gnx`**.
   No usarlo como global: este resolver no reenvía consultas públicas.
   Search domain `mesh.gnx` es opcional; los nombres completos no lo necesitan.
3. Conectar la app Android al mismo tailnet e instalar **sólo la CA pública**
   GNX (`root.crt`) para un navegador que la acepte. Nunca copiar la clave CA.
4. Con Wi-Fi apagado, abrir ambos HTTPS sin advertencias; después comprobar
   recuperación e identidad tras reinicio del host. No usar `hosts` como prueba.

## Estado comprobado — 2026-09-02

| Gate | Resultado |
|---|---|
| Rust | 5 tests, Clippy, release y RustSec pasan |
| Imagen DNS | UDP/TCP, apex, wildcard anidado, AAAA vacío y sin upstream pasan; publicación loopback verificada y retirada |
| Nodo WSL | Quadlet activo; `NeedsLogin`; reaplicar retorna `FAILED ACCESS_ENROLLMENT_REQUIRED` |
| Infraestructura previa | Ambos HTTPS siguen respondiendo 200 con TLS validado |
| Enrolamiento / DNS productivo | Pendientes de la clave; no se publica DNS con una IP inventada |
| SaaS / Android / reboot | Pendientes; no se han modificado DNS ni políticas del SaaS |

Estado privado: `/var/lib/gnx/access`, root-only. El respaldo USB existente
**no lo incluye**. Faltan respaldo/restauración y verificar renovación/expiración
de la identidad del proveedor. No clonar el estado entre máquinas. Sin sesión
Windows, el arranque conserva la limitación de la tarea actual del host.

## Dependencias y referencias

- Tailscale 1.102.3: [código y licencia BSD-3-Clause](https://github.com/tailscale/tailscale/tree/v1.102.3), [clave por archivo](https://tailscale.com/docs/reference/tailscale-cli/up), [split DNS](https://tailscale.com/docs/reference/dns-in-tailscale).
- Pi-hole FTL 6.7: [código y licencia EUPL-1.2](https://github.com/pi-hole/FTL/tree/v6.7), [configuración DNS](https://docs.pi-hole.net/ftldns/configfile/). Se usa el motor sin UI, DHCP, NTP ni registro de consultas; se conservan atribuciones de las imágenes.
