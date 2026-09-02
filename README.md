# GNX

Base Windows-first para un ejecutable Rust pequeño, gobernado por archivos de
configuración y con integraciones externas detrás de contratos GNX.

## Primer corte

Sólo cubre cuatro resultados:

1. leer y validar la configuración;
2. comprobar el host Windows;
3. instalar o localizar el cliente de mesh nativo;
4. conectar el nodo local al `control_server` y reportar su estado real.

El modo `create` añade un único control plane autocontenido. `join` nunca lo
inicia. Linux, Proxmox, routing, proxy y UI quedan fuera hasta cerrar este corte.

## Reglas

- Rust compone el flujo; la configuración contiene la intención.
- Comandos, módulos, archivos y servicios propios usan nombres GNX.
- El proveedor actual sólo aparece dentro del adaptador, manifest, SBOM,
  licencias y atribuciones.
- No hay daemon VPN propio, fallback oculto ni secretos en Git, argv o logs.
- Un gate fallido nunca produce `READY`.

## Documentos

- [Arquitectura](docs/architecture.md)
- [Auditoría](docs/audit.md)
- [ADR de plataforma mesh](docs/decisions/0001-mesh-platform.md)
- [ADR de identidad y endpoint](docs/decisions/0002-mesh-identity-and-endpoint.md)
