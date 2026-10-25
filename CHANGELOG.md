# Changelog

## 0.0.1 — versión inicial

### Añadido
- CLI y servicio Windows Rust con control por pipe local protegido.
- Identidad y WSL dedicadas, sin adoptar la instalación GnX anterior.
- Verificación de inputs offline mediante SHA-256.
- Base Fedora bootc fijada por digest y cuatro imágenes OCI locales: Tailscale,
  Pi-hole, Caddy y Proxmox VE.
- Quadlets concretos, target systemd y logging nativo a journald.
- Definiciones MSI/EXE WiX y publicación HTTPS local con confianza explícita.

### Límites
- Primera versión; no confundir capacidades implementadas con aceptación en el host.
- Despliegue app/CLI a LXC y OIDC Pocket ID/NetBird/RustFS pendientes.
- Imagen propia derivada y capacidades nativas de bootc en LXC pendientes;
  exportar un filesystem OCI para WSL no equivale a bootc upgrade/rollback.
