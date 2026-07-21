# Cierre del PoC de Quetzalcoatl como MVP real

## Veredicto

El repositorio ya tiene un **PoC real de un solo nodo controller**. No es únicamente código de demostración: instala, reinicia, persiste identidad, levanta Proxmox, ejecuta OpenTofu y prueba Garage y Forgejo.

Sin embargo, **el MVP definido por la arquitectura todavía no está cerrado**, porque la promesa actual es:

```text
1 controller + 2 members + clúster Proxmox quorate
```

Actualmente el código detiene explícitamente cualquier segundo nodo con:

```text
MEMBER_INCREMENT_DEFERRED
```

Además, `PersistedRole` sólo admite `Controller`.

---

# 1. Lo imprescindible para cerrar el MVP funcional

## 1.1 Demostrar que la red de tres nodos es viable

Este debe ser el siguiente trabajo. Antes de programar el proceso de incorporación de miembros, hay que ejecutar la prueba A-06 sobre tres hosts Windows reales.

La prueba actual con runners de GitHub no sirve como validación para Corosync porque obtuvo:

```text
Tailscale por DERP
RTT aproximado: 64–73 ms
```

El contrato debe exigir:

```text
Tailscale directo, sin DERP
0 % pérdida
RTT < 5 ms
controller ↔ member-1
controller ↔ member-2
member-1 ↔ member-2
```

También deben comprobarse:

- Resolución estable de nombres.
- Relojes sincronizados.
- TCP 22 entre nodos Proxmox.
- TCP 8006 entre nodos Proxmox.
- UDP 5405–5412 para Corosync.
- MTU funcional.
- Ningún puerto Proxmox publicado en Windows.

### Decisión crítica

El MVP debe declarar explícitamente que los tres equipos están:

```text
En el mismo sitio, LAN o red física de baja latencia
```

No conviene prometer un clúster Proxmox entre endpoints Windows geográficamente distribuidos. Tailscale puede proporcionar conectividad, pero no garantiza siempre la latencia ni la ruta directa que Corosync necesita.

Si tres hosts reales no pasan este gate, no debe implementarse I2 todavía. Primero habría que revisar la topología del clúster.

---

## 1.2 Implementar el camino `member`

El grueso del trabajo está localizado en `gnx-service`. No es necesario rehacer el instalador.

### Estado persistente

Actualmente:

```rust
pub enum PersistedRole {
    Controller,
}
```

Debe soportar:

```rust
pub enum PersistedRole {
    Controller,
    Member,
}
```

El estado de un member debe guardar, como mínimo:

- ID Tailscale propio.
- IP Tailscale propia.
- Hostname lógico propio.
- ID del controller.
- IP actual del controller.
- Hostname del controller.
- Tailnet.
- Etapa de reconciliación.
- Estado de incorporación al clúster.

El rol debe decidirse una sola vez y nunca cambiar automáticamente.

### Descubrimiento de rol

Debe reemplazarse `MEMBER_INCREMENT_DEFERRED` por una matriz de decisión explícita:

| Peers encontrados | Resultado |
|---:|---|
| 0 | Controller |
| 1 con exactamente un `gnx-controller-*` | Primer member |
| 2 con exactamente un controller | Segundo member |
| Más de 2 | `TOPOLOGY_UNSUPPORTED` |
| Ningún controller identificable | `TOPOLOGY_UNSUPPORTED` |
| Más de un controller | `TOPOLOGY_UNSUPPORTED` |

También deben excluirse:

- El propio nodo.
- Peers expirados.
- Sidecars `tag:quetzalcoatl-service`.
- Equipos sin el tag exacto del producto.

---

## 1.3 Implementar el join seguro de Proxmox

Éste es el blocker B-06.

El member debe:

1. Levantar su Proxmox local.
2. Configurar su identidad y contraseña local.
3. Comprobar API, SSH, Corosync, nombres, tiempo y ruta Tailscale directa.
4. Ejecutar:

```text
pvecm add <controller-tailnet-ip> --link0 <member-tailnet-ip>
```

5. Verificar que `ring0_addr` utiliza la IP de Tailscale.
6. Confirmar que el nodo aparece dentro del clúster.
7. Confirmar que el clúster tiene quorum.
8. Eliminar cualquier material temporal de autenticación.

La credencial del controller no puede aparecer en:

- Argumentos de proceso.
- Logs de Windows.
- Logs de systemd.
- Archivos persistentes sin cifrar.
- Historial shell.
- `state.json`.
- OpenTofu state.

El canal debe seguir el patrón existente:

```text
DPAPI
→ stdin o canal temporal
→ archivo efímero en /run con permisos 0600
→ ejecución
→ eliminación
```

El join también debe ser idempotente:

- Si el nodo ya está unido, verificar y continuar.
- Si el controller está temporalmente offline, quedar en un error reanudable.
- Si el join quedó incompleto, no crear otro clúster ni cambiar de controller.
- Un reinicio debe retomar la misma etapa.

---

## 1.4 Bloquear completamente OpenTofu en members

No basta con no llamarlo desde el flujo normal. Debe existir una denegación explícita antes de lanzar cualquier proceso.

En un member debe verificarse que:

```text
No se crea workspace OpenTofu
No existe terraform.tfstate
No se materializan credenciales PVE para OpenTofu
No se ejecuta init, plan o apply
No se crean LXC
No se reconvergen Garage o Forgejo
```

El resultado esperado es:

```text
OpenTofu instalado como parte del payload
OpenTofu operacionalmente no aplicable
```

En `gnx status --json` puede reportarse:

```json
{
  "role": "member",
  "components": {
    "opentofu": "not_applicable"
  }
}
```

Garage y Forgejo también deben aparecer como `not_applicable`, no como `pending`.

---

## 1.5 Completar `gnx status` para members

Los dos members deben terminar con un estado equivalente a:

```json
{
  "overall": "ready",
  "stage": "READY",
  "role": "member",
  "controller": "gnx-controller-...",
  "cluster": {
    "joined": true,
    "quorate": true
  },
  "components": {
    "service": "ready",
    "wsl": "ready",
    "podman_machine": "ready",
    "kvm": "ready",
    "tailscale": "ready",
    "tailscale_serve": "ready",
    "proxmox": "ready",
    "opentofu": "not_applicable"
  },
  "services": {
    "garage": "not_applicable",
    "forgejo": "not_applicable"
  }
}
```

También deben existir errores operativos claros:

```text
CONTROLLER_UNAVAILABLE
TAILSCALE_DIRECT_PATH_REQUIRED
CLUSTER_NETWORK_PREFLIGHT_FAILED
PVE_JOIN_FAILED
TOPOLOGY_UNSUPPORTED
MEMBER_OPENTOFU_DENIED
```

---

# 2. Prueba de aceptación real del MVP

El MVP queda funcionalmente cerrado solamente cuando se realiza la siguiente prueba completa.

## 2.1 Instalación

Usar tres equipos Windows 11 limpios y ejecutar el mismo artefacto:

```text
QuetzalcoatlSetup.exe
```

Instalarlos secuencialmente:

```text
Host 1 → controller
Host 2 → member
Host 3 → member
```

No debe ser necesario editar archivos manualmente entre instalaciones.

## 2.2 Validación del clúster

Desde los tres nodos:

```text
pvecm nodes
pvecm status
```

Debe observarse:

```text
3 nodos
1 controller lógico GNX
2 members
clúster quorate
ring0_addr sobre Tailscale
```

## 2.3 Persistencia y reinicios

Reiniciar individualmente:

1. Member 1.
2. Member 2.
3. Controller.

Después de cada reinicio:

- El servicio vuelve automáticamente.
- El rol no cambia.
- No se repite la detección inicial.
- El member continúa unido.
- El clúster recupera quorum.
- Garage y Forgejo siguen siendo únicos.
- `gnx status --json` vuelve a `READY`.

## 2.4 Seguridad

Auditar los tres hosts:

- Sin auth keys en texto plano.
- Sin contraseñas en argumentos o logs.
- Sin secretos en Compose.
- Sin secretos en `state.json`.
- Sin workspace OpenTofu en members.
- ACL de ProgramData limitada a SYSTEM y al servicio.
- Named Pipe rechazando clientes no autorizados.
- Cero listeners Proxmox publicados en Windows.

---

# 3. Lo necesario para convertirlo en un MVP pilotable

El cierre técnico anterior demuestra el producto. Para entregarlo a usuarios piloto se necesitan algunas capas adicionales.

## 3.1 Firma de código

Deben firmarse:

```text
QuetzalcoatlSetup.exe
Quetzalcoatl.msi
gnx.exe
gnx-service.exe
gnx-host-preflight.exe
```

Para laboratorio interno puede aceptarse temporalmente una excepción. Para entregar el instalador a terceros, la firma es prácticamente obligatoria por SmartScreen, UAC y confianza operativa.

---

## 3.2 Preflight de capacidad

El runtime actualmente fija aproximadamente:

```text
6 vCPU
8 GiB RAM para WSL
2 GiB swap
100 GiB de disco para Podman Machine
```

Pero HostPreflight sólo comprueba Windows, elevación, virtualización, WSL y Podman.

Falta validar:

- CPU lógica disponible.
- RAM total.
- RAM libre.
- Espacio libre.
- Capacidad de virtualización anidada.
- Recursos adicionales para Windows y los LXC.

No basta con comprobar que KVM existe si después el host no puede reservar los recursos requeridos.

---

## 3.3 Diagnóstico mínimo

La CLI actualmente sólo tiene:

```text
gnx configure
gnx status
gnx status --json
```

Para un piloto debe existir un camino de soporte mínimo:

```text
gnx diagnostics bundle
```

El bundle debería incluir, siempre redactado:

- Estado GNX.
- Etapa actual y último error.
- Estado del servicio Windows.
- Estado de WSL y Podman Machine.
- Unidades systemd.
- Estado Quadlet.
- Estado Tailscale sin claves.
- Estado Proxmox y del clúster.
- Versiones y hashes instalados.
- Logs acotados sin secretos.

No hace falta construir una plataforma completa de observabilidad.

---

## 3.4 Operación documentada

Debe existir un README operativo corto que explique:

- Hardware soportado.
- Preparación de Tailscale.
- Tags y `tagOwners`.
- ACL mínima.
- MagicDNS y certificados HTTPS.
- Orden de instalación.
- Ejecución de `gnx configure`.
- Cómo reconocer el estado `READY`.
- Cómo reanudar después de un error.
- Cómo obtener diagnósticos.
- Cómo actualizar.
- Cómo desinstalar sin destruir datos.
- Cómo limpiar manualmente un entorno de laboratorio.

---

## 3.5 Ciclo completo de instalación

Antes de llamar al resultado MVP pilotable deben probarse:

- Instalación limpia.
- Reinicio requerido y reanudación.
- Instalación silenciosa.
- Major upgrade.
- Repair o reinstalación.
- Desinstalación normal.
- Reinstalación preservando o detectando estado existente.
- Fallo controlado cuando ya existe una Podman Machine ajena.

No es necesario implementar todavía un comando destructivo `purge`, pero sí debe existir un procedimiento de recuperación documentado.

---

# 4. Lo que debe permanecer fuera del MVP

Para cerrar el proyecto sin ampliar indefinidamente el alcance, deben quedar fuera:

- Tauri Tray.
- Headscale.
- Forgejo Runner.
- Alta disponibilidad.
- Elección manual del controller.
- Promoción automática de members.
- Cuarto nodo.
- Instalaciones iniciales concurrentes.
- Multi-cloud o múltiples runtimes.
- Backend S3 para OpenTofu.
- API gRPC.
- UI avanzada del Command Center.
- Framework general de migraciones.
- Telemetría remota.

La interfaz del MVP puede seguir siendo:

```text
Setup.exe
+ gnx configure
+ gnx status
```

---

# 5. Definición final de terminado

```text
El mismo Setup.exe instala secuencialmente tres hosts Windows 11 compatibles.

El primero converge como controller.

Los otros dos convergen como members.

Los tres forman un clúster Proxmox quorate sobre rutas Tailscale directas.

Sólo el controller ejecuta OpenTofu y mantiene Garage y Forgejo.

Los roles, secretos y estado sobreviven reinicios.

No existen puertos Proxmox publicados en Windows.

Los artefactos están firmados y el operador puede diagnosticar fallos sin acceder a secretos.
```

---

# 6. Orden recomendado de ejecución

```text
A-06: demostrar la red real
→ I2: implementar member
→ aceptación de tres hosts y reinicios
→ firma, preflight de recursos y diagnóstico
→ release MVP
```

---

# 7. Checklist ejecutivo

## Bloqueadores del MVP funcional

- [ ] Tres hosts reales pasan la prueba de red.
- [ ] Las rutas Tailscale son directas.
- [ ] RTT y pérdida cumplen el contrato de Corosync.
- [ ] Existe el rol persistente `Member`.
- [ ] El descubrimiento de controller es determinista.
- [ ] El join de Proxmox es seguro e idempotente.
- [ ] El clúster de tres nodos alcanza quorum.
- [ ] OpenTofu está explícitamente bloqueado en members.
- [ ] Garage y Forgejo sólo existen en el controller.
- [ ] Los tres nodos sobreviven reinicios sin cambiar de rol.
- [ ] No se exponen puertos Proxmox en Windows.
- [ ] No aparecen secretos en logs, argumentos o estados.

## Requisitos del MVP pilotable

- [ ] EXE, MSI y binarios firmados.
- [ ] Preflight de CPU, RAM y disco.
- [ ] Comando `gnx diagnostics bundle`.
- [ ] Guía operativa.
- [ ] Instalación limpia probada.
- [ ] Upgrade probado.
- [ ] Repair o reinstalación probada.
- [ ] Desinstalación y recuperación documentadas.

---

## Resultado esperado

Cuando todos los puntos anteriores estén cerrados, Quetzalcoatl dejará de ser únicamente un PoC técnico de controller y podrá presentarse como un **MVP real, instalable, reproducible y operable de clúster GNX de tres nodos sobre Windows 11**.
