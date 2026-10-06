# v0.1.72 — release del fix Pocket y actualización instalada

## Build e instalación exacta

- 66 tests Rust pass (12 protocolo + 46 host + 8 setup); contratos estáticos pass.
- Build canónico release/MSI/Burn pass; inputs offline y rootfs sellado verificados.
  WiX sin warnings/errors; warning debug LNK4098 conocido, no fatal.
- Burn instalado **exit 0**; `gnx version` = **0.1.72**.
- `gnx upgrade --confirm` elevado concluyó **NodeReady**, seguido de `gnx status`
  correcto. No se parcheó el payload después de construir este candidato.
- [runtime-checks.jsonl](runtime-checks.jsonl): **53 checks canónicos pass +
  `payload_current` pass**, 54 registros. Incluyen PWA anónima/CSRF/HTTPS,
  sesiones, servicios privados, OIDC, Management/Signal gRPC, relay y STUN.
- Un sondeo concurrente durante la aplicación observó `rustfs_gateway_https`
  fail transitorio; la comprobación final posterior a la actualización pasa y
  devuelve `ready=true`. No se sustituyó el gate ni se declaró listo antes.

## Preservación durante upgrade

Comparación antes/después, sin imprimir valores secretos:

- SID de `gnxnodesvc`, hash del secreto DPAPI y CA Windows iguales.
- Owner OIDC/Valkey, cookie secret y CA del gateway iguales.
- Claves RustFS, claves Pocket y cliente OIDC PWA iguales.
- Identidad de los directorios persistentes RustFS/Pocket/NetBird (dev/inode)
  iguales; el contenedor compute tampoco fue recreado.
- Esto no constituye una auditoría de cada objeto S3 ni de todos los datos de
  todas las aplicaciones. No se desinstaló ni borró el guest compartido.

## Ronda de desinstalación: qué se probó

- `tests/installer/test-uninstall.mjs`: contrato completo MSI/Burn/cleanup pass.
- `tests/installer/test-msi-conditions.ps1`: evalúa condiciones del **MSI
  compilado**, sin Install/DoAction ni custom actions. Seis escenarios pass;
  [resultados](msi-conditions.jsonl).
- El log MSI del upgrade real confirma que `CleanupOnUninstall` y
  `CleanupFreshResidue` se omiten por condición en el MSI viejo/nuevo.
- **No se ejecutó una desinstalación destructiva real en host desechable.**
  Hacerlo en este host borraría identidad, CA, credenciales, distro y datos.
  No se presenta esta ronda como aceptación end-to-end de borrado/reinstalación.

## Autenticación

[isolated-oidc.jsonl](isolated-oidc.jsonl): 16 checks pass con OAuth2 Proxy/Valkey
reales y issuer RS256/TLS aislado, ahora con `email_verified=false`. Conserva
nonce, CSRF, cookies, grupo/revocación, logout/server replay y fallo cerrado.

El diagnóstico Pocket real previo confirmó el rechazo del correo no verificado;
el hotfix abrió la PWA real en Firefox (Nodo operativo/100%), documentado en
[la evidencia anterior](../v0.1.71/pocket-subject-hotfix.md). v0.1.72 empaqueta
esa misma corrección. No se infiere una nueva aceptación personal completa de
los tests aislados: logout/replay/revocación con Pocket real, cold boot/host
limpio y peers reales no se repitieron aquí. Los warnings Proxmox
ebtables/pending changes siguen fuera del alcance.

## Publicación

El bundle nuevo se publica en el dataset existente: el intento preferente en el
Space no produjo una descarga verificable. El dataset sí entrega la descarga
pública íntegra, validada por streaming SHA-256/tamaño, sin MSI duplicado.
Ver [artefacto](../../../releases/v0.1.72/ARTIFACT.md). Fuente, notas y hashes se
publican como snapshot comprometido en el Space, con revisión padre explícita;
no se reescribe historia ni se purgan objetos históricos.
