# Operar GNX

## Orden seguro

```mermaid
flowchart TD
    C[Editar gnx.toml] --> P[compute apply]
    P --> R[controller apply]
    R --> A[access configure]
    A --> V{Servicio aprobado}
    V -->|no| O[Aprobar svc:compute\ny repetir access apply]
    V -->|sí| D[Configurar Split DNS\ngnx → IP Pi-hole]
    O --> D
    D --> G[access dns]
```

- `compute status`, `controller status` y `access dns` son los gates de salud.
- `READY` significa que el gate terminó; cualquier fallo conserva `FAILED <ETIQUETA>`.
- La clave de enrolamiento entra sólo por prompt oculto, nunca por archivo de
  configuración, argumentos, logs o Git.
- `controller.autonomous_ca = true` genera la raíz dentro de
  `/var/lib/gnx/controller`. No la instala en Windows ni en otro cliente.
- El nombre canónico con TLS administrado es `compute.<tailnet>.ts.net`.
  `compute.gnx` depende de Pi-hole y, para HTTPS, del CA autónomo.

## Gate de release

```text
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
packaging/windows/build.ps1
```

El build es válido sólo si genera `gnx.exe`, `gnx` y sus SHA-256.
