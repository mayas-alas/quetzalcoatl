# GNX

**Tu infraestructura privada, en una sola caja.**

GNX convierte un host Linux (o Windows vía WSL2) en un nodo privado y
verificable: red, cómputo y superficie HTTPS convergen con tres comandos y
cero secretos en configuración. Cada operación termina en `READY` o en
`FAILED <ETIQUETA>` — sin estados intermedios, sin puertas ocultas.

## Qué obtienes

- **Acceso privado sin exponer puertos.** Tailscale entrega el transporte y
  TLS administrado para `*.ts.net`; Pi-hole responde la zona `.gnx` por
  Split DNS. Nada escucha en la red pública.
- **Cómputo con salud comprobada.** El ciclo de vida del nodo (Proxmox en
  contenedor) se aplica y se verifica con gates reales: identidad, API,
  uptime.
- **HTTPS propio, opcional y explícito.** Un CA autónomo firma la ruta
  `.gnx` para operación e investigación local; confiar en él siempre es tu
  decisión, nunca un efecto colateral.
- [ADDRESS] **auditable.** Imágenes fijadas por digest, permisos 0600/0700
  verificados, claves sólo por prompt oculto. Un gate fallido [PERSON_NAME]
  [PERSON_NAME] nunca se oculta como éxito.

## Cómo se usa

```text
gnx access      # Tailscale, Services y Pi-hole para Split DNS .gnx
gnx compute     # ciclo de vida y salud del servicio de cómputo
gnx controller  # entrada HTTP y CA autónomo opcional para HTTPS .gnx
```

Windows es un puente delgado: valida la misma configuración y delega la acción
en WSL2. Sin servicio, sin bandeja, sin estado que mantener.

## [PERSON_NAME]

1. Copiar `config/gnx.example.toml` a `gnx.toml` y sustituir el FQDN de ejemplo.
2. [ADDRESS] bundle generado por `packaging/windows/build.ps1`.
3. Ejecutar, en orden: `gnx compute apply`, `gnx controller apply` y
   `gnx access configure`.
4. Aprobar `svc:compute` en Tailscale si el reporte lo solicita.
5. En DNS del tailnet, añadir el nameserver reportado y restringirlo a `gnx`.

Reparar es volver a aplicar: todas las operaciones son idempotentes.
Diagnosticar es preguntar: `compute status`, `controller status` y `access dns`
son los mismos gates de la instalación.

La arquitectura completa, las decisiones y la operación están en
[arquitectura](docs/arquitectura.md), [operar](docs/operar.md) y
[decisiones](docs/decisions/).

## Licencia

GNX usa `AGPL-3.0-only`; las dependencias conservan sus licencias y
atribuciones. La rama histórica `legacy` queda archivada y separada, sin
modificaciones.

## Release gate

```powershell
# Construir
.\packaging\windows\build.ps1

# Validar contrato sobre artefactos release
.\packaging\validate.ps1 -DistPath dist
```

`validate.ps1` ejecuta los 3 contract-smoke (`WINDOWS_CONTRACT`,
`LINUX_CONTRACT`, `ARGUMENTS_CONTRACT`) contra los binarios de `dist/`.
Salida: `READY VALIDATION` o `FAILED <ETIQUETA>`.
