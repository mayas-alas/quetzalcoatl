# Auditoría: rootfs gestionado por instalador

## Decisión

La UX objetivo es que el instalador descargue automáticamente el rootfs y lo
verifique antes de mutar el host. La descarga debe ser una capacidad del
instalador, no una instrucción manual al usuario.

## Estado actual

Hoy `install.ps1` exige `-Rootfs` y `-RootfsSha256`; no descarga nada. La ruta
legacy que descargaba Ubuntu fue retirada correctamente.

## Riesgo de una implementación ingenua

No basta con añadir `Invoke-WebRequest` antes de `gnx-setup.exe`:

- El manifest podría estar manipulado antes de comprobar su firma.
- Un SHA pasado por argv no es una raíz de confianza.
- Una URL mutable o con redirecciones puede servir otro artefacto.
- El archivo puede escribirse parcialmente o consumir espacio ilimitado.
- El rootfs descargado puede quedar expuesto en rutas temporales o logs.
- Reintentos concurrentes pueden dejar estado incompleto.

## Diseño de remediación

1. El manifest firmado contiene `rootfs`:
   - `url`: URL HTTPS permitida;
   - `sha256`: digest esperado;
   - `size_bytes`: límite exacto/máximo;
   - `filename`: nombre fijo.
2. Un comando/verificador nativo valida la firma del manifest **antes** de usar
   la URL.
3. El instalador descarga a staging privado y con límite de bytes.
4. Se rechazan URL no HTTPS, credenciales embebidas, redirects fuera del host
   permitido, archivos sparse/reparse y tamaños inesperados.
5. Se calcula SHA256 sobre el archivo completo y se compara con el valor firmado.
6. Solo después se invoca la rutina existente de setup/import.
7. Se conserva el rootfs hasta completar health; se elimina en éxito o rollback.
8. Evidencia sanitizada: URL/versión/hash/bytes/resultado, nunca secretos ni
   tokens.

## Casos que deben auditarse

- URL ausente o manifest sin sección `rootfs`.
- Manifest no firmado, firma incorrecta o schema no soportado.
- HTTPS inválido, host no permitido, redirect externo o URL con userinfo.
- Respuesta HTTP distinta de 200.
- Content-Length ausente, mayor al límite o discrepante.
- Descarga truncada, hash incorrecto o tar inválido.
- Poco espacio en staging.
- Interrupción durante descarga, hash, staging, import y reboot.
- Dos instalaciones concurrentes.
- Reintento tras descarga incompleta.
- rootfs ya existente con hash distinto.
- Cambio de manifest entre verificación y consumo.
- Cancelación y limpieza sin borrar una instalación válida previa.
- Ausencia de red: resultado `ACTION_REQUIRED`, nunca `READY`.

## Próximo cambio seguro

No se debe habilitar aún una descarga basada en una URL suministrada por argv o
en una variable de entorno. Falta definir y fijar el origen HTTPS oficial y
extender el verificador de manifest para autenticarlo antes de descargar.
