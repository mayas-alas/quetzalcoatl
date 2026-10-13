# Máquinas dedicadas y publicación de servicios

**Estado: objetivo aprobado, no implementado ni aceptado.** La autoridad de
despliegue es el [contrato config-driven](config-driven-platform.md) y
`.AGENTS/40-config-driven-deployments.md`. Toda corrección sigue fuente →
build → instalación → lifecycle instalado → evidencia; no hotfixes.

## Objetivo actual: Continuity de negocio

Migrar el Continuity `business` que hoy vive en la máquina compartida a un
LXC dedicado. Debe conservar exactamente su scope, prefijo, historial y
semántica de **single writer**. El LXC nuevo contiene Continuity y el proxy
oficial NetBird BYOP, enlazado al cluster privado de la cuenta del operador.

La máquina compartida conserva y sirve como dependencia reutilizada a RustFS,
Pocket ID, NetBird management/server/dashboard, la CA y el gateway/Caddy.
No se crea otro management, dashboard, gateway, control plane ni login para
`app.gnx`; la restauración de `app.gnx` queda fuera de alcance.

## Inventario de ownership

| Retener | Reutilizar | Retirar, sólo tras migración probada |
| --- | --- | --- |
| Identidad Windows/DPAPI/DACL, Named Pipe, CA, datos e historial business, receipts y evidencia. | Adaptador LXC y lifecycle común; enrolamiento mesh protegido; auth/proxy genéricos; RustFS/S3, Pocket, management y gateway existentes. | Guest AI antiguo y sus perfiles/artefactos; ramas de lifecycle específicas de AI; cualquier control plane duplicado. |

Retirar significa detener por el lifecycle autorizado, verificar que ningún
servicio lo referencia, preservar discos/receipts/historial y liberar recursos
sin borrar datos salvo decisión explícita. No copiar identidad de red ni
arrancar un segundo escritor durante la transición.

## Migración mínima y segura

1. Inventariar ownership, VMID, disco/rootfs, scope, prefijo, writer,
   credenciales limitadas, health y dependencias. Tomar el estado durable antes
   de cambiar ownership.
2. Preparar el LXC sin privilegios con Debian fijado y **Podman oficial 6.1.3**.
   Esta versión corrige CVE-2026-94603. El Podman 6.1.1 del pool Debian no se
   usa sin revisión de seguridad explícita; no se acepta como sustituto.
3. Importar una única imagen Continuity verificada (la misma imagen aprobada,
   sin reconstruirla por scope) y configurar el mismo `scope=business`,
   spool/prefijo e historial lógico. Usar RustFS existente
   con una credencial limitada; no inventar bucket, destino S3 ni copiar una
   clave admin.
4. Enrolar el LXC como peer/proxy nuevo en el cluster NetBird account/private,
   con token por secret store/archivo protegido. No copiar estado mesh. Caddy
   permanece delante como gateway y la CA existente se conserva.
5. Detener el writer antiguo mediante el lifecycle, comprobar que no quedan
   escrituras en vuelo y activar el nuevo. Verificar materialización,
   replicación, acceso/auth y ausencia de un segundo writer antes de emitir el
   receipt aplicado.
6. Validar restart/upgrade conservando datos, identidad, CA y credenciales.
   Sólo después retirar el guest AI antiguo y cualquier rama AI que ya no tenga
   referencias, con revisión de tests y evidencia.

No se anuncia una sintaxis CLI nueva: la aplicación debe usar las operaciones
versionadas y el lifecycle ya instalado. No se crea shell desde la PWA ni un
listener HTTP local.

## Requisitos del proxy BYOP

El componente es la imagen oficial NetBird fijada por digest, con `Pull=never`
cuando el bundle sea offline. Debe mostrar cluster de tipo `account`, privado,
con proxy conectado y sincronización completa antes de declararse listo. El
token, dominio y certificados son inputs protegidos; nunca aparecen en argv,
logs, Git, URLs o perfiles públicos. No se abre exposición pública ni se
confunde el proxy con Pocket ID.

La configuración exacta del cluster, dominio privado, imagen/digest
Continuity, base Debian/Podman 6.1.3, recursos y credencial RustFS limitada son
inputs pendientes. El destino S3 aprobado para releases/inputs offline también
está pendiente de endpoint, bucket/prefijo, CA y credencial del operador; no se
debe inventar.

## Historia AI (conservada, no alcance actual)

El diseño anterior separaba `private-services` (RustFS, Pocket, management y
Continuity business) de un guest `ai` para Voice/Oryx/OmniRoute/Continuity ai y
BYOP. El candidato v0.1.78 creó/preparó ese guest, pero no obtuvo aceptación de
servicios, HTTPS, login o inferencia. Véase [evidencia AI](validation/ai-services/README.md).

También se conserva la inspección del source Continuity en
`C:/Users/mayas/continuity`, commit `d5c8886`, la metadata histórica del proxy
NetBird v0.80.0 y el hallazgo de dashboard **No clusters available**. El
enrolador BYOP no llegó a conectarse al lifecycle. Estos datos son evidencia
histórica/incompleta, no autorización para reactivar AI ni para restaurar
`app.gnx`.

El registro técnico histórico también conserva los digests inspeccionados del
proxy v0.80.0 (índice multi-arch
`sha256:6d6655b3f13f837d80cc44878033866713932d3c5c1381ed869af9542b523bc5` y
Linux amd64
`sha256:735c68a60624c72c315e7c3612bfe760f8c75251cf327afd2b5e0a60c9cdab6e`),
la observación de que `proxy-enroll.run` no estaba conectado al instalador y
los límites de scope/single-writer de Continuity. Son metadatos de inspección,
no una imagen importada, un cluster conectado ni una aceptación.

## Aceptación pendiente

La aceptación futura debe probar ownership/idempotencia, preservación de
scope/prefijo/historial, single writer, Podman 6.1.3, cluster account/private,
proxy conectado, health real de Continuity, RustFS/Pocket/gateway reutilizados,
TLS/auth, restart/upgrade y retiro seguro del guest AI. Un contenedor `Up`, un
dry-run, un digest de registry o una pantalla de login no sustituyen esa
evidencia. No declarar migración, release o publicación hasta completar los
gates y verificar hashes/readback en los destinos aprobados.
