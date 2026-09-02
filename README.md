# GNX

GNX es una base de infraestructura privada local: un ejecutable Rust pequeño
conecta Windows a una mesh propia, mientras WSL aloja el control y los servicios
mediante Quadlet. Archivos de configuración expresan la intención y contratos
GNX encapsulan las integraciones.

## Primer corte

El cliente cubre cuatro resultados en Windows x86_64:

1. leer y validar la configuración;
2. comprobar el host Windows;
3. verificar e instalar un paquete MSI local del cliente de mesh;
4. conectar el nodo local al `control_server` y reportar su estado real.

La operación local añade dos servicios, separados del binario cliente:

| Dirección | Función | Estado comprobado |
|---|---|---|
| `https://mesh.gnx` | Control plane y consola | TLS, conexión y misma identidad tras reboot Windows |
| `https://proxmox.mesh.gnx` | Primer servicio de cómputo | TLS, login API y reinicio del servicio; `8006` interno |

Ambas direcciones resuelven mediante `hosts` en este Windows y DNS privado
desde Android, confirmado por el operador. El respaldo cifrado en USB
cubre el control plane; faltan respaldo de cómputo y restauración operativa.

La [capa de acceso Android](docs/access.md) tiene nodo WSL, DNS privado y dos
comandos: `gnx access configure` pide la clave al humano sin eco; `gnx access dns`
muestra los campos del nameserver y valida MTU/DNS/HTTPS/política. El uplink WSL
se configura a MTU 1500 antes de iniciar la VPN. Faltan datos móviles y reboot.
No sustituye la infraestructura existente.

## Reglas

- Rust compone el flujo; la configuración contiene la intención.
- Comandos, módulos, archivos y servicios propios usan nombres GNX.
- Las dependencias se identifican en adaptadores, imágenes, manifest, SBOM,
  licencias y documentación técnica. Se conserva el alias de servicio
  `proxmox.mesh.gnx` elegido por el operador.
- No hay daemon VPN propio, fallback oculto ni secretos en Git, argv o logs.
- Un gate fallido nunca produce `READY`.

## Uso

```text
cargo build --release --locked
gnx.exe install --config config/gnx.toml --release runtime/release.toml
gnx.exe doctor --config config/gnx.toml
gnx.exe connect --config config/gnx.toml
gnx.exe access configure
gnx.exe access dns
gnx.exe credentials control
gnx.exe credentials compute
```

`release.toml` referencia un MSI local, su SHA-256 y licencia. No contiene
URLs de descarga. Para enrolamiento desatendido, `connect` acepta
`--setup-key-file`; nunca acepta la clave como valor.
Ese contrato corresponde al cliente mesh Windows. Acceso usa sólo el prompt
humano y toma `access.toml` junto al EXE; no solicita archivos de claves.
`credentials` recupera las dos cuentas locales desde DPAPI: Enter revela en
pantalla temporal y otro Enter oculta. Requiere consola y el usuario Windows
original, sin redirección ni portapapeles. No grabar ni transcribir la terminal.

`packaging/windows/build.ps1` produce `dist/windows/gnx.exe`. Con los tres
insumos del cliente genera un bundle instalable; sin ellos produce sólo el
bundle de desarrollo y no simula que la dependencia esté lista.

Para registrar esa CLI en el host, ejecutar
`packaging/windows/install-host.ps1` desde PowerShell **como administrador**.
Verifica el SHA-256, instala en `C:/Program Files/GNX`, conserva la configuración
existente y actualiza el PATH. Retira el servicio y las carpetas de la instalación
anterior a un respaldo restringido en `C:/ProgramData/GNX/retired-host`;
no borra discos ni modifica la VPN o el WSL actuales. Abrir una terminal nueva
y comprobar `gnx access dns`. Las actualizaciones usan el mismo instalador.

## Documentos

- [Arquitectura](docs/architecture.md)
- [Auditoría](docs/audit.md)
- [Control plane local y rutinas del host](docs/control.md)
- [Primer servicio de cómputo](docs/compute.md)
- [Acceso privado desde Android](docs/access.md)
- [ADR de plataforma mesh](docs/decisions/0001-mesh-platform.md)
- [ADR de identidad y endpoint](docs/decisions/0002-mesh-identity-and-endpoint.md)
