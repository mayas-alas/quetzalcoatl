# 0001 — CA autónomo: renovación automática, raíz inmutable, confianza deliberada

- **Estado:** propuesta
- **Contexto:** AGENTS.md §3, `docs/operar.md` (experimental)
- **Tags:** ca, controller, renewal, secrets

## Contexto

El CA autónomo (`runtime/controller/ca.sh`) firma los certs server de Caddy para
HTTPS `.gnx`. El nombre canónico con TLS administrado sigue siendo
`compute.<tailnet>.ts.net` (Tailscale Services): **el CA no es source of truth**.
Tailscale provee transporte y TLS `*.ts.net`; `gnx-dns` (dnsmasq) mantiene la
autoridad DNS mínima de la zona `.gnx`. El CA `.gnx` es exclusivamente para
operación/depuración local.

`ca.sh` ya es idempotente: regenera el *server cert* cuando expira en <30 días o
cambian los SAN (`cmp -s` sobre `domains`). La **raíz** (`root.key`,
RSA-3072, `pathlen:0`, `nameConstraints:DNS:.gnx`) se genera una sola vez y no se
revisa. La confianza de la raíz en Windows (`trust-ca.ps1`) es manual y requiere
admin — es una decisión de seguridad, no de operación.

El operador no quiere intervención manual (`apply`) ni que el CA sea source of
truth, y prefiere **integraciones inteligentes** sobre nuevos paradigmas.

## Decisión

1. **Raíz inmutable.** La raíz (`root.key`/`root.crt`) se genera una sola vez
   (si no existe) y **nunca** se revoca ni rota. 10 años de vida, `pathlen:0`.
   La raíz permanece local en el state directory y firma únicamente certificados
   `.gnx`. Si se compromete, el recovery es manual (borrar `pki/`, regenerar y
   volver a distribuir confianza). CRL/OCSP y rotación automática de raíz no
   forman parte del alcance actual.

2. **Renovación automática del server cert (no raíz).** Se añade un *timer*
   systemd `gnx-ca-renew.timer` (diario, `Persistent=true`) que ejecuta
   `gnx controller apply`. `ca.sh` rota `tls/server.{key,crt}` bajo su política
   idempotente. No requiere intervención manual. El CA sólo se ejecuta si
   `controller.autonomous_ca = true`.

3. **Confianza deliberada, no automática.** `trust-ca.ps1` permanece manual
   (admin explícito). No se automatiza la confianza de raíz en Windows ni en
   clientes. Un `controller status` verifica que el server cert esté vigente y
   que `pki.gnx` resuelva a la IP del controller.

4. **Discovery de la raíz vía `gnx-dns`.** `pki.gnx` → IP del controller
   (registro dnsmasq existente en `records()`). El cliente obtiene la raíz
   pública vía HTTP desde el propio `.gnx`: `curl http://pki.gnx/root.crt`.
   Para validar HTTPS `.gnx`, el cliente usa después esa raíz explícitamente.
   No se instala confianza de forma automática.

## Integración existente aprovechada

- `ca.sh` ya valida idempotencia (`domains` vs `pending`, `checkend 2592000`).
- `Caddyfile` ya reverse_proxy a compute vía `tls_trust_pool /etc/gnx/upstream-ca.crt`.
- `records()` ya emite `address=/pki.gnx/{access_ip}` cuando `autonomous_ca`.
- `controller.rs::status()` ya verifica HTTPS `.gnx` con `--cacert root.crt`.

## Consecuencias

- **Ventajas:** menos código (sin CRL ni rotación automática de raíz); renovación
  transparente del certificado de servidor y menor superficie operativa.
- **Desventajas:** sin revocación granulacional; una raíz comprometida requiere
  redeploy manual. Aceptable porque Tailscale es source of truth y `.gnx` es
  operacional, no productivo.

## References

- `runtime/controller/ca.sh` — generación idempotente de raíz y server cert.
- `runtime/controller/Caddyfile` — `@PRIVATE_SITES@`, reverse_proxy a compute.
- `src/controller.rs` — `private_sites()`, `status()` verifica TLS `.gnx`.
- `docs/operar.md` — orden seguro, experimental CA.
- AGENTS.md §3 — claves/URLs no entran en Git/argv/logs.
