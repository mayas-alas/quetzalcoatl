# Gateway local de agentes

**Estado:** frontera propuesta de tooling; no es parte del producto instalado.

Los agentes se ejecutan en CLIs externas y alcanzan un único gateway GNX por
loopback, por ejemplo `http://127.0.0.1:31415`. El repositorio no implementa su
motor, sus modelos ni un ciclo de coordinación dentro del binario `gnx`.

## Contrato

- `/v1/*` expone la superficie compatible para modelos, chat, respuestas,
  mensajes y embeddings.
- `/*` permite adaptadores adicionales sólo mediante rutas explícitas; nunca es
  un proxy abierto.
- El listener se liga únicamente a `127.0.0.1`. Exponerlo en LAN requiere otra
  decisión de seguridad.
- Cada CLI aporta autenticación mediante su configuración nativa o variables de
  entorno. Las credenciales no se derivan de imágenes ni se guardan en Git.
- El gateway devuelve uso normalizado cuando el proveedor lo informa; la
  ausencia de métricas se representa como desconocida, no como cero.

## Separación

Este gateway no es Headscale, `gnx-netd` ni la LocalAPI de
`/run/gnx/netd.sock`. Tampoco se empaqueta en el EXE de Windows, los Quadlets o
el instalador Linux. Su único propósito es dar transporte local a agentes de
desarrollo sin contaminar la arquitectura del runtime.

Antes de automatizarlo se prueban cuatro cosas: bind exclusivo a loopback,
autenticación, matriz de rutas permitidas y reporte de uso por CLI.
