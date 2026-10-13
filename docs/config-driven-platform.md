# Plataforma config-driven: contrato operativo acotado

Fecha: 2026-10-09. **Diseño aprobado; implementación IN PROGRESS.** La
aceptación/release requiere build, instalación, aplicación por el lifecycle
instalado, restart, evidencia live/browser cuando corresponda y publicación
verificada por el coordinador. Este documento no acredita una migración
completada.

Normas obligatorias: [despliegues config-driven](../.AGENTS/40-config-driven-deployments.md).

## Alcance aprobado

La entrega actual es migrar el Continuity existente de negocio a un LXC
dedicado. Debe conservar `scope=business`, su prefijo, historial y **un único
escritor**. El LXC nuevo ejecuta Continuity y el proxy oficial NetBird BYOP,
unido a un cluster privado de la cuenta del operador. Se reutilizan RustFS,
Pocket ID, NetBird management/server/dashboard, la CA y el gateway/Caddy
existentes; no se duplica ningún control plane.

La restauración o ampliación de `app.gnx` queda fuera de este cambio. El guest
AI anterior, Voice, Oryx y OmniRoute son trabajo histórico/incompleto: deben
retirarse de forma segura, no reactivarse ni convertirse en una nueva entrega.

## Inventario: conservar, reutilizar y retirar

| Acción | Alcance aprobado |
| --- | --- |
| **Conservar** | Identidad Windows dedicada, DPAPI/DACL, elevación, Named Pipe seguro, CA, estado/receipts y lifecycle común. Conservar el historial y datos de `business`. |
| **Reutilizar** | Adaptador LXC, mesh/enrolamiento protegido y los adaptadores genéricos de autenticación/proxy obtenidos del trabajo AI, RustFS/S3 existente, Pocket ID, NetBird management y gateway. El proxy BYOP oficial es un componente del nuevo LXC, no otro management. |
| **Retirar después de comprobar referencias y pruebas** | Ramas y allowlists de lifecycle específicas de AI, perfiles/artefactos del guest AI y scaffolding duplicado. Primero debe existir el recorrido genérico de migración y su evidencia; no se borra historia ni datos por slimming. |
| **No hacer** | Restaurar `app.gnx`, crear otro control plane, copiar identidades mesh, arrancar dos escritores para `business`, adoptar un guest ajeno, shell/root desde la PWA o anunciar comandos no implementados. |

No hay un DAG/workflow engine nuevo. Rust sólo aporta seguridad Windows, IPC,
validación y coordinación genérica; perfiles, Quadlets, imágenes, red,
persistencia, checks y referencias de secretos permanecen fuera de Rust.

## Bundle y admisión genéricos

La forma objetivo es un bundle versionado; describe un contrato, no una
promesa de que un perfil arbitrario ya sea cargable:

```text
profile.json                 # id, schema, resources, desired state, ownership
images.json                  # OCI digests, archives, hashes and sizes
quadlets/*.container         # Quadlets concretos, no plantillas .in
quadlets/*.network
config/<service>/...         # configuración externa sin secretos
policy/...                   # red/acceso dentro de capacidades aprobadas
checks.json                  # verificadores tipados, no shell libre
manifest.json                # fuente, revisión, hashes, límites y requisitos
```

Admitir sólo archivos confinados, claves Quadlet permitidas, UID/capacidades,
redes, volúmenes, CPU/RAM/disco, logging, digests y referencias de secretos
autorizadas. Rechazar traversal, links/devices, mounts host, `Exec*`,
`PodmanArgs`, `Privileged`, units `.service` arbitrarias, secretos inline y
shell enviado por frontend. Un dry-run no es sandbox.

El receipt registra ID lógico, VMID, ownership, revisión/digests, configuración
deseada/aplicada, recursos, identidad mesh, dependencias reutilizadas, estado,
último error y evidencia. Para Continuity registra además scope/prefijo y el
escritor autorizado. Conserva el alias/ownership heredado hasta una migración
probada.

## Cambios mínimos del lifecycle común

El recorrido único es:

```text
validate ownership/prefix → plan + lock → prepare LXC/Podman
→ independent NetBird enrollment → reuse RustFS/Pocket/management/gateway
→ apply Quadlets → verify writer/replication/access → applied receipt
```

- La migración detiene el escritor antiguo antes de activar el nuevo; nunca
  escribe simultáneamente en `continuity/business`.
- Cada LXC tiene hostname e identidad mesh propios. Un peer existente conserva
  su estado; sólo un peer nuevo usa la clave reusable protegida. Nunca se copia
  `tailscaled.state` ni otra identidad.
- Restart/upgrade conserva discos, scope, prefijo, datos, CA, credenciales
  protegidas e identidad del peer. No re-enrola silenciosamente.
- El lifecycle debe respetar `desired state=stopped`, diferenciar fallo de
  observación de health fallida y no traducir ninguno a `ready` o aceptación.
- Para retirar AI: comprobar ownership, dependencias y evidencia; detener por
  el lifecycle autorizado, preservar discos/historial y liberar recursos sólo
  con una decisión explícita. No hacer rollback destructivo ni prune global.

## Podman y dependencias

Podman **6.1.3 oficial** es obligatorio para el LXC nuevo; es la versión
aprobada con la corrección de CVE-2026-94603. El paquete 6.1.1 disponible en el
pool Debian no se usa como sustituto vulnerable sin revisión de seguridad
explícita. El input de instalación, firma/hash y base Debian fijada deben
quedar verificados antes del build.

RustFS sigue siendo el almacenamiento existente; no se inventa bucket,
prefijo, credencial ni destino S3. La credencial del nuevo guest debe estar
limitada a `continuity/business`. La clave admin del guest privado no se copia.
Para publicación de releases/inputs offline falta todavía el destino S3
aprobado (endpoint, bucket/prefijo, CA y credencial); ese input debe venir del
operador antes de subir nada.

El proxy oficial NetBird usa el cluster **account/private** existente. Su token
y configuración entran por secret store/archivo protegido, nunca argv, URL,
logs, perfil público o Git. Caddy sigue siendo el gateway HTTPS existente y
NetBird no sustituye Pocket ID ni cambia el login de `app.gnx`.

## Observabilidad, build y publicación

CLI y PWA, cuando estén implementados para este recorrido, consumirán la misma
proyección: operación correlacionada, logs sanitizados con cursor/límites y
separación entre pipe ocupado, timeout, servicio ausente, health fallida y
aceptación. No se anuncia una sintaxis nueva.

Fuente → inputs verificados → build → instalación → aplicación por lifecycle
instalado → restart/live/browser evidence → publicación/readback por hash. No
hotfix, EXE viejo, guest editado a mano ni dry-run como aceptación. Publicar
sólo en HF y en el S3 existente una vez que el operador confirme el destino;
no purgar historial ni tratar blobs S3 como registry OCI.

## Estado histórico del trabajo AI

El perfil/guest AI previo y [su evidencia](validation/ai-services/README.md)
son históricos e incompletos: el candidato v0.1.78 instaló/preparó un guest,
pero no acreditó servicios, HTTPS, login o inferencia. La inspección anterior
registró Continuity `business` en el LXC compartido y un diseño separado
`continuity/ai`; ese diseño ya no es el alcance aprobado. También quedó
registrado que el dashboard NetBird mostraba **No clusters available** y que el
enrolador BYOP no estaba conectado al lifecycle. Ninguno de esos hechos es una
aceptación actual ni autoriza reactivar AI.

## Gates pendientes

1. Inventariar receipt, VMID, writer, scope/prefijo, datos y dependencias del
   LXC actual; obtener inputs aprobados de imagen Continuity, Podman 6.1.3,
   recursos, credencial limitada RustFS y cluster account/private.
2. Implementar y probar sólo el lifecycle genérico de migración, retiro seguro
   y single-writer; después revisar referencias/tests antes de eliminar ramas
   AI específicas.
3. Aplicar el LXC dedicado, comprobar cluster `account`, proxy conectado,
   Continuity ready/replicado, acceso por gateway y upgrade/restart sin pérdida.
4. Retirar el guest AI únicamente tras esa evidencia. Publicar artefactos sólo
   cuando exista el destino S3 operador-aprobado y se verifique readback/hash.

No hacer un big-bang rewrite. Mantener la auth aprobada, los servicios privados
y la evidencia histórica; registrar siempre qué capacidad se entregó y cuál
gate sigue pendiente.
