# ADR-0003: acceso operativo independiente

**Estado:** aceptado; Android confirmado por el operador, datos móviles/reboot pendientes.
**Fecha:** 2026-09-02

Se añade un nodo Tailscale SaaS en WSL y un resolver Pi-hole privado. Android
alcanza así la entrada HTTPS existente sin necesitar resolver primero el
control plane privado. NetBird conserva su función actual; no se reemplazan
`gnx.exe`, el cliente Windows, el control plane ni las identidades existentes.

`ops/access` es el núcleo Rust compartido por `gnx access configure/apply/dns`;
`runtime/access` declara el corte y contiene
plantillas Quadlet/DNS. Nombres propios por capacidad: `gnx-access`, `gnx-dns`.
El uplink WSL (`eth0`, MTU 1500) se declara en configuración y un servicio de
arranque lo aplica antes de la VPN; el túnel mantiene MTU 1280.
No se desarrolla otro protocolo VPN ni un gestor genérico de proveedores.

DNS responde el apex y wildcard de `mesh.gnx`; Caddy sólo sirve los dos sitios
declarados. Usar split DNS, no nameserver global. Search domains no sustituyen
DNS, transporte, políticas ni confianza en la CA local.

La identidad persiste en WSL; el humano introduce la clave en un prompt sin eco.
GNX gestiona el archivo temporal `0600` en tmpfs y su eliminación; no pide
archivos al usuario ni acepta secretos en argv. `dns` muestra los campos exactos
del formulario SaaS, con IP pendiente si no existe conexión real.
La política SaaS, la confianza Android, el acceso por datos móviles y el reboot
son gates separados. Sin enrolamiento no se asigna una IP ficticia ni se publica
el resolver productivo. [Operación y evidencia](../access.md).

Fuera del corte: DuckDNS, Termux, cliente VPN propio, exit node, rutas de
subred, otro cliente Windows, DNS global y administración SaaS genérica.
