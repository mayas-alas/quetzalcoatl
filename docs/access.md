# Acceso privado desde Android

**Corte:** un nodo de acceso en WSL; DNS privado y los dos HTTPS existentes.
No sustituye el cliente Windows ni el control plane local.

Android usa Tailscale SaaS para alcanzar `gnx-access`; Pi-hole responde
`mesh.gnx` y `*.mesh.gnx` con la IP de ese nodo. Caddy sigue terminando TLS y
enviando cómputo a `8006` interno. DNS no crea servicios ni certificados wildcard.
La VPN, la resolución y la confianza TLS son tres requisitos distintos.

## Aplicar

Desde la raíz del repo, usando el EXE actualizado (no otro `gnx` en el PATH):

```powershell
.\dist\windows\gnx.exe access configure
.\dist\windows\gnx.exe access dns
```

La configuración por defecto es `access.toml` junto al EXE; `--config` permite
seleccionar otro archivo del mismo corte. `configure` pide al **humano** una
Auth key one-off, no efímera, sólo cuando hace falta enrolar. Entrada oculta;
Enter vacío cancela. No admite claves por argumentos, archivos ni redirección.
No enviarlas al agente. No se crea un archivo de credenciales en Windows.

GNX limita la copia temporal a un archivo `0600` en tmpfs del contenedor,
la elimina tras éxito o fallo y limpia el buffer Rust. Un corte forzado puede
requerir reiniciar el contenedor para descartar residuos RAM. La identidad del
nodo sí persiste. No necesita otro cliente Windows ni nuevo UAC.

`gnx access apply` reaplica sin pedir credenciales. `gnx access dns` sólo
consulta el estado; devuelve fallo explícito si falta conexión o salud local.
El núcleo `ops/access` se comparte con el EXE; no se duplica el flujo.

`access.toml` declara `uplink = "eth0"` y `uplink_mtu = 1500`.
`apply` instala y habilita `gnx-access-network.service` antes del nodo VPN;
el túnel conserva MTU 1280. `dns` falla con `UPLINK_MTU` si el valor observado
no coincide. Configuraciones anteriores deben añadir esos dos campos.

Valida el corte, arranca Quadlet, enrola con su archivo temporal, conserva ID/IP y genera
DNS con la IP observada. Las imágenes están fijadas por digest. No acepta DNS
ni rutas del SaaS, no anuncia subredes, no activa SSH ni exit node. Reaplicar
no vuelve a enrolar un nodo conectado; un cambio de identidad falla cerrado.
`READY access-local` exige MTU, DNS UDP/TCP desde la red del host, ambos HTTPS
con CA/nombre válidos y una política recibida no vacía;
**no acredita acceso remoto**. La salida externa de autenticación no se publica.
`ACCESS_POLICY_EMPTY` señala una política que no permite tráfico entrante;
una política no vacía tampoco prueba todos los permisos del teléfono.

## Campos de «Add nameserver»

| Campo | Valor que muestra `gnx access dns` |
|---|---|
| Nameserver | IP VPN observada; `PENDING` si aún no está conectado |
| Restrict to domain / Split DNS | ON |
| Domain | `mesh.gnx`, sin asterisco |
| Use with exit node | OFF |
| Search domain (fuera de ese formulario) | `mesh.gnx`, opcional |

No usar la IP WSL, `127.0.0.1` ni una IP de ejemplo como nameserver del teléfono.

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
| Rust | 9 tests de acceso; Clippy y release pasan |
| Entrada humana | Consola real: sonda no secreta sin eco, rechazada por formato; entrada redirigida y valores en argv rechazados |
| Custodia temporal | stdin, tmpfs, permiso `0600` y eliminación tras éxito/fallo pasan con sonda no secreta |
| Imagen DNS | UDP/TCP, apex, wildcard anidado, AAAA vacío y sin upstream pasan; publicación loopback verificada y retirada |
| Nodo WSL | Enrolado por el humano; identidad persistida y nameserver `100.91.239.31` |
| Infraestructura previa | Ambos HTTPS siguen respondiendo 200 con TLS validado |
| DNS / política SaaS | Split DNS `mesh.gnx`; regla por usuario hacia DNS 53, HTTPS 443 y CRL 80; política recibida comprobada |
| Transporte | MTU WSL 1280 reproducía esperas; con 1500 ambos cuerpos HTTPS completos pasan tres veces desde Windows por VPN |
| Android | El operador confirmó ambos dominios; el peer observado estaba en Wi-Fi. Datos móviles y ausencia de advertencias TLS aún no comprobados |
| Persistencia | Servicio de uplink habilitado y ordenado antes del nodo; reboot completo todavía pendiente |

Estado privado: `/var/lib/gnx/access`, root-only. El respaldo USB existente
**no lo incluye**. Faltan respaldo/restauración y verificar renovación/expiración
de la identidad del proveedor. No clonar el estado entre máquinas. Sin sesión
Windows, el arranque conserva la limitación de la tarea actual del host.

## Dependencias y referencias

- Tailscale 1.102.3: [código y licencia BSD-3-Clause](https://github.com/tailscale/tailscale/tree/v1.102.3), [clave por archivo](https://tailscale.com/docs/reference/tailscale-cli/up), [split DNS](https://tailscale.com/docs/reference/dns-in-tailscale).
- [MTU de WSL y encapsulado VPN](https://tailscale.com/docs/install/windows/wsl2).
- Pi-hole FTL 6.7: [código y licencia EUPL-1.2](https://github.com/pi-hole/FTL/tree/v6.7), [configuración DNS](https://docs.pi-hole.net/ftldns/configfile/). Se usa el motor sin UI, DHCP, NTP ni registro de consultas; se conservan atribuciones de las imágenes.
