# ADR-0003: acceso operativo independiente

**Estado:** aceptado; conexión remota pendiente de validación.  
**Fecha:** 2026-09-02

Se añade un nodo Tailscale SaaS en WSL y un resolver Pi-hole privado. Android
alcanza así la entrada HTTPS existente sin necesitar resolver primero el
control plane privado. NetBird conserva su función actual; no se reemplazan
`gnx.exe`, el cliente Windows, el control plane ni las identidades existentes.

`ops/access` es el operador Rust; `runtime/access` declara el corte y contiene
plantillas Quadlet/DNS. Nombres propios por capacidad: `gnx-access`, `gnx-dns`.
No se desarrolla otro protocolo VPN ni un gestor genérico de proveedores.

DNS responde el apex y wildcard de `mesh.gnx`; Caddy sólo sirve los dos sitios
declarados. Usar split DNS, no nameserver global. Search domains no sustituyen
DNS, transporte, políticas ni confianza en la CA local.

La identidad persiste en WSL; la clave entra por archivo y nunca por argv.
La política SaaS, la confianza Android, el acceso por datos móviles y el reboot
son gates separados. Sin enrolamiento no se asigna una IP ficticia ni se publica
el resolver productivo. [Operación y evidencia](../access.md).

Fuera del corte: DuckDNS, Termux, cliente VPN propio, exit node, rutas de
subred, otro cliente Windows, DNS global y administración SaaS genérica.
