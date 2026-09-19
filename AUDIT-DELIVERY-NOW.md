# GNX — auditoría de entrega inmediata

Fecha: 2026-09-19  
Alcance: candidato Windows actual en `dist/`

## Veredicto

**CLI entregable para ejecución controlada: SÍ.**  
**Runtime instalado y READY en host: NO demostrado todavía.**

No se debe presentar `doctor` con `ACTION_REQUIRED` como fallo del binario: en este host no existe/inicia `GNXRuntime`.

## Evidencia ejecutada

| Control | Resultado |
|---|---|
| `cargo test --locked --all-targets` | PASS: 18 unit/integration + 2 setup, 1 ignorado por requerir Linux root |
| `dist/gnx.exe` | Ejecuta; contrato JSON válido |
| `gnx.exe doctor` | `ACTION_REQUIRED / BROKER_UNAVAILABLE / exit 2` |
| `gnx-setup.exe --check` | `ACTION_REQUIRED / SETUP_SOURCE_NOT_FOUND / exit 2`; no mutó host |
| `manifest.json` | `sealed` |
| `manifest.sig` | Presente, 64 bytes |
| Hashes de artefactos | Coinciden con manifest para los 6 artefactos |
| Rootfs manifest/metadata | SHA256 coincide |
| Descarga dinámica Ubuntu | Retirada; el provisionador legacy falla cerrado |

## Qué entregar ahora

Para una demo/entrega de negocio, entregar el directorio `dist/` junto con el
rootfs exacto cubierto por el manifest:

```text
dist/
  gnx.exe
  gnx-service.exe
  gnx-setup.exe
  gnx-linux
  gnx-linux-bundle.tar
  gnx-linux.run
  manifest.json
  manifest.sig
  rootfs.metadata.json
  assets/
rootfs.tar   # externo al dist; SHA256 debe coincidir con manifest.rootfs_sha256
```

No entregar binarios sueltos renombrados ni reconstruir el manifest después de
copiar archivos.

## Camino de ejecución en host

1. Abrir PowerShell elevado.
2. Ejecutar `install.ps1` con `manifest.json`, `manifest.sig` y el `rootfs.tar`
   cuyo SHA256 coincide.
3. Aceptar `ACTION_REQUIRED`/reboot si lo solicita.
4. Tras reinicio, comprobar:

```powershell
& .\gnx.exe doctor
& .\gnx.exe status
```

Solo se comunica `READY` si `doctor` y `status` lo devuelven y el servicio,
WSL, persistencia e identidad fueron observados independientemente.

## Riesgo residual explícito

La aceptación end-to-end Windows aún no está probada en este host: `doctor`
confirma que el broker/runtime no está disponible. Por tanto, la etiqueta
correcta para negocio es **Windows candidate / controlled preview**, no
**production-ready release**.
