# GNX

Base Windows-first para un ejecutable Rust pequeño, gobernado por archivos de
configuración y con integraciones externas detrás de contratos GNX.

## Primer corte

Sólo cubre cuatro resultados en Windows x86_64:

1. leer y validar la configuración;
2. comprobar el host Windows;
3. verificar e instalar un paquete MSI local del cliente de mesh;
4. conectar el nodo local al `control_server` y reportar su estado real.

El control plane debe existir antes. Su despliegue, Linux, Proxmox, routing,
proxy y UI quedan fuera hasta cerrar este corte.

## Reglas

- Rust compone el flujo; la configuración contiene la intención.
- Comandos, módulos, archivos y servicios propios usan nombres GNX.
- El proveedor actual sólo aparece dentro del adaptador, manifest, SBOM,
  licencias y atribuciones.
- No hay daemon VPN propio, fallback oculto ni secretos en Git, argv o logs.
- Un gate fallido nunca produce `READY`.

## Uso

```text
cargo build --release --locked
gnx.exe install --config config/gnx.toml --release runtime/release.toml
gnx.exe doctor --config config/gnx.toml
gnx.exe connect --config config/gnx.toml
```

`release.toml` referencia un MSI local, su SHA-256, licencia y SBOM. No contiene
URLs de descarga. Para enrolamiento desatendido, `connect` acepta
`--setup-key-file`; nunca acepta la clave como valor.

## Documentos

- [Arquitectura](docs/architecture.md)
- [Auditoría](docs/audit.md)
- [ADR de plataforma mesh](docs/decisions/0001-mesh-platform.md)
- [ADR de identidad y endpoint](docs/decisions/0002-mesh-identity-and-endpoint.md)
