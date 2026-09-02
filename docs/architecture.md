# Arquitectura GNX

**Corte:** Windows-first, Rust-first.

## Resultado único

`gnx.exe` lee configuración, valida Windows, verifica e instala un MSI local del
cliente de mesh y conecta el nodo. Sólo reporta `READY` cuando el estado
observado coincide con la configuración.

```mermaid
flowchart LR
    C["gnx.toml"] --> R["gnx.exe"]
    R --> H["host"]
    R --> M["mesh"]
    M --> E["control_server"]
```

El proveedor actual de mesh es NetBird. Su nombre, comandos y formatos quedan
encapsulados en el adaptador; se conservan en licencias, manifest, SBOM y
diagnósticos técnicos, no en la interfaz pública ni en la taxonomía.

## Contrato mínimo

```text
gnx install --config <archivo>
gnx connect --config <archivo>
gnx doctor --config <archivo>
```

```toml
version = 1

[node]
mode = "join" # create | join

[mesh]
control_server = "https://mesh.gnx"
```

- El control plane es un prerrequisito y GNX no lo despliega en este corte.
- El endpoint se conserva exactamente y nunca cae a un servicio distinto.
- El login es interactivo o usa `--setup-key-file`; la clave nunca viaja en argv.

## Orden de implementación

1. Parsear y validar `gnx.toml` en Rust.
2. Implementar `doctor` de Windows sin mutaciones.
3. Implementar `connect` contra el cliente nativo.
4. Implementar `install` idempotente desde un release local verificado.
5. Empaquetar el mismo binario como `gnx.exe`.

No se abre otra plataforma o workload antes de cerrar estos cinco puntos.

## Taxonomía

```text
gnx/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                 # composición y salida
│   ├── config.rs               # único esquema público
│   ├── app/
│   │   ├── install.rs
│   │   ├── connect.rs
│   │   └── doctor.rs
│   ├── port/
│   │   ├── host.rs
│   │   └── mesh.rs
│   └── adapter/
│       ├── host.rs             # Windows en el primer corte
│       └── mesh.rs             # dependencia externa aislada
├── config/
│   └── gnx.example.toml
├── runtime/
│   └── release.example.toml    # MSI, versión, digest, licencia y SBOM
├── packaging/
│   └── windows/
│       └── build.ps1
└── tests/
    ├── contract.rs
    └── windows.rs
```

`app` contiene los tres casos de uso. `port` define lo que necesitan. `adapter`
traduce la dependencia y Windows. No existen módulos raíz llamados como
proveedores, versiones, daemons o servicios concretos.

## Fuera del corte

Control plane, WSL, contenedores, Linux, Proxmox, routing, proxy, UI, HA y
actualización automática. Cada uno necesitará un caso probado antes de añadir
módulos o configuración.
