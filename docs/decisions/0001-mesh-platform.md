# ADR-0001: plataforma mesh detrás de GNX

**Estado:** aceptado para el primer corte  
**Fecha:** 2026-09-02

## Decisión

GNX usa un cliente mesh mantenido por su proveedor e instalado de forma nativa
en Windows. El control plane local se prepara aparte en WSL con `ops/control`.
Sigue siendo un prerrequisito de `gnx connect`. GNX no
implementa VPN, criptografía, relay ni traversal.

NetBird es la implementación elegida hoy, pero no forma parte del contrato
público. Sólo el adaptador traduce:

```text
control_server -> management-url
estado externo -> estado GNX
```

Comandos, configuración, módulos y servicios propios usan nombres GNX o nombres
de capacidad. La identidad legal del proveedor permanece visible en paquetes,
licencias, SBOM y diagnóstico técnico.

## Alcance

- Un peer nativo en el host Windows.
- Endpoint HTTPS exacto, sin fallback.
- Login interactivo o setup-key file; ningún secreto por argv.
- Recuperación de identidad y conexión tras reboot.

Linux cliente, workloads, routing, publicación de aplicaciones, HA y
actualizaciones automáticas no pertenecen a esta decisión. El despliegue local
del control plane se limita al procedimiento de [operación](../control.md).

## Gates

| ID | Evidencia |
|---|---|
| `M-01` | `gnx connect` usa el endpoint configurado y refleja el fallo real. |
| `M-02` | Windows mantiene una sola identidad tras reboot. |
| `M-03` | El binario no ofrece una ruta parcial para iniciar el control plane. |
| `S-01` | Paquete, versión, digest y licencia están fijados. |
| `S-02` | No aparecen secretos en Git, argv, entorno, logs o evidencia. |

## Fuentes

- [Self-hosting de NetBird](https://docs.netbird.io/selfhosted/selfhosted-quickstart)
- [CLI de NetBird](https://docs.netbird.io/get-started/cli)
- [Instalación en Windows](https://docs.netbird.io/get-started/install/windows)
