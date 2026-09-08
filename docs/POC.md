# Quetzalcoatl — Contrato de aceptación del PoC

Versión documental: 1.0, 7 de septiembre de 2026. Complementa [arquitectura.md](arquitectura.md). **Todos los gates de este documento están pendientes de ejecución.** Finalizar estos documentos no equivale a finalizar el PoC.

## 1. Resultado obligatorio

El operador aplica GNX, conecta un cliente al tailnet y abre **`https://proxmox.gnx`** con certificado confiable. Puede iniciar sesión en el **Proxmox VE real de Dockurr**, cuyo contenedor administra **Podman Quadlet + systemd**. Configuración, identidad y storage sobreviven a reinicios y reaplicaciones.

En la misma revisión, otro hostname publica un servicio externo que conserva su propio lifecycle. Detener ese servicio no rompe Proxmox, DNS ni la identidad de red.

La decisión Dockurr/Proxmox/Quadlet/`proxmox.gnx` es un requisito de negocio cerrado. Un impedimento de plataforma se diagnostica y registra; no se cambia de appliance, se sustituye por una página de prueba ni se retira HTTPS para declarar éxito.

## 2. Alcance y laboratorio

Se prueba **un nodo por ejecución**, con la misma revisión GNX y los mismos digests en dos perfiles obligatorios. No se requiere tener ambos nodos GNX activos a la vez.

| ID de entorno | Función | Perfil de referencia |
|---|---|---|
| L | Nodo Linux nativo | Ubuntu 24.04 LTS x86-64, systemd, cgroup v2, Podman/Quadlet, KVM y TUN utilizables |
| W | Nodo Windows anfitrión | Windows 11 x64 + WSL2; distro GNX con Ubuntu 24.04, systemd y las capacidades anteriores |
| CW | Cliente Windows remoto | Otro equipo/VM, con cliente oficial Tailscale; sin acceso administrativo a la distro GNX |
| CL | Segundo cliente remoto | Linux con Tailscale, en otra red respecto del nodo; permite DNS, TLS y pruebas de conectividad reproducibles |
| X | Servicio externo | Endpoint HTTP privado con respuesta identificable y administración independiente de GNX |

Ambos clientes consultan el resolver del sistema; no se añade un resolver DoH de navegador ni un exit node en el perfil base. Se fija también navegador y versión. Android queda como prueba adicional, sin sustituir CL ni ampliar automáticamente soporte.

El tailnet ya existe. El operador puede configurar manualmente DNS y permisos específicos. Se prueba con clientes autorizados y con un cliente o identidad de prueba sin acceso. No se modifica infraestructura ajena al laboratorio.

**Dentro:** Access Tailscale/CoreDNS, Control Caddy, Compute Dockurr/Proxmox, CLI Linux/puente Windows, bootstrap reproducible, estado persistente, una ruta externa, diagnóstico, idempotencia y recuperación.

**Fuera:** catálogo, adaptación de WatchTower, segunda malla, scheduler, HA, CI/CD, backups de workloads, aprovisionamiento GNX de LXC/VM, instaladores comerciales, tray, actualización automática y Windows como servicio desatendido sin login. Las capacidades internas de Proxmox no quedan certificadas por un login exitoso.

El gesto de recuperación Windows forma parte del contrato: tras reboot, inicia sesión el propietario y arranca la distro; se mantiene una sesión WSL activa durante la prueba. No se afirma arranque automático sin login. Una implementación futura que cambie ese gesto deberá agregar su propio gate.

## 3. Congelar la ejecución

Antes de ejecutar los gates, crear un registro de ejecución fuera de los archivos de intención, por ejemplo `docs/evidence/<run-id>/LOCK.md`, con:

| Campo | Contenido obligatorio |
|---|---|
| Revisión | Commit GNX; si hay cambios locales, diff y hashes de los archivos revisados, incluidos los nuevos |
| Plataforma | IDs L/W/CW/CL/X, versiones Windows/WSL/kernel/distro, CPU/RAM/disco y resultado de KVM/TUN/cgroups |
| Herramientas | Rust, systemd, Podman, runtime OCI y herramientas de prueba realmente usadas |
| Imágenes | Registro, versión legible y digest OCI exacto de Tailscale, CoreDNS, Caddy y `docker.io/dockurr/proxmox` |
| Dockurr | Referencia de fuente asociada o trazabilidad disponible de la imagen; entrypoint, mounts y requisitos inspeccionados |
| Red | Identidad/IP GNX, subred interna elegida, política split DNS y permisos acotados del tailnet |
| Confianza | Huellas de las CA públicas de Caddy y Proxmox, y nombres TLS esperados |
| Externo | Endpoint, respuesta testigo, responsable de arranque/parada y versión de X |
| Procedimientos | Comandos exactos de bootstrap, apply, diagnóstico, prueba y recuperación |

No se inventan digests ni se toma `latest` como referencia reproducible. El digest histórico del repositorio puede investigarse, pero no prueba por sí solo disponibilidad, procedencia ni compatibilidad actuales. La imagen final debe seguir siendo Dockurr/Proxmox.

Los valores concretos de hosts y credenciales pertenecen a la ejecución, no a esta especificación. El registro contiene referencias públicas, nunca contraseñas, tokens, cookies ni claves privadas.

## 4. Orden de implementación

| Corte | Entrega observable | Pruebas que lo cierran |
|---|---|---|
| 0. Base verificable | Manifiesto de versiones y preflight; comprobar pronto KVM/WSL y requisitos de Dockurr | G00–G01 |
| 1. Runtime y Compute | Configuración tipada, CLI, Quadlet Proxmox, API local autenticada y estado persistente | G02–G04 |
| 2. Access | Identidad Tailscale, CoreDNS y split DNS real desde clientes | G05–G07 |
| 3. Control | `https://proxmox.gnx`, las dos conexiones TLS y controles negativos | G08–G11 |
| 4. Integración y errores | Ruta externa, recuperación de configuración y protección de secretos | G12–G14 |
| 5. Windows y cierre | Puente Windows, reboot en ambos perfiles y comprobación final conjunta | G15–G18 |

El bloqueo de W no impide avanzar los cortes disponibles en L. El cierre global sí requiere los dos perfiles. Las pruebas parciales de un corte se conservan como evidencia parcial; no convierten el corte en PASS.

### Contrato del bootstrap

El bootstrap instala o comprueba dependencias, crea directorios privados, fija imágenes, instala CLI/Quadlets y describe exactamente cómo arrancar. Se admiten pasos administrativos manuales reproducibles. No descarga un instalador comercial ni importa código antiguo sin revisar.

`gnx plan` no modifica el host. `gnx apply` usa la intención validada y afecta solo recursos GNX. Cada comando tiene timeout, código de salida y error con fase. `status` y `doctor` son observación, no una vía silenciosa para reinstalar servicios.

Las comprobaciones de Compute usan endpoints reales de Proxmox: autenticación mediante `/api2/json/access/ticket` o un token dedicado, estado de `/api2/json/nodes/<node>/status` e inventario de storage. Los nombres y campos exactos se verifican contra la versión fijada. No se registran tickets o cabeceras Authorization.

## 5. Matriz de gates

PASS exige todos los resultados de la fila. **PENDIENTE** significa no ejecutado; **FAIL**, ejecución con resultado incorrecto; **BLOCKED**, falta de un requisito externo identificado. Esos estados de prueba son distintos de READY/FAILED/ACTION_REQUIRED del runtime.

Salvo las filas que especifican un perfil, las pruebas de nodo se repiten en L y W. CW y CL prueban cada perfil sucesivamente, usando la IP GNX de esa ejecución.

| Gate | Prueba y procedimiento | Resultado requerido |
|---|---|---|
| G00 | Revisar LOCK y disponibilidad de imágenes | Referencias concretas, reproducibles y misma familia Dockurr/Proxmox; ninguna etiqueta mutable ni evidencia de otra revisión |
| G01 | Preflight en L/W antes de modificar estado | KVM utilizable —no solo archivo existente—, TUN, cgroup v2, generación Quadlet, recursos y rutas sin colisiones; error accionable cuando falta un requisito |
| G02 | Ejecutar plan, apply y entradas inválidas | Plan sin mutaciones; aplicación con bloqueo; CLI/JSON/códigos consistentes; rechaza hosts repetidos, configuración inválida y URLs con secretos antes de publicar |
| G03 | Arrancar Compute y consultar API con autenticación | Imagen fijada de Dockurr bajo `gnx-compute.service`; login/nodo esperados, estado e inventario de storage útiles; credencial incorrecta rechazada |
| G04 | Reaplicar sin cambios y comprobar estado testigo | Se conserva identidad, CA, credencial y archivo testigo de storage; no recrea contenedores ni reinicia servicios sanos; el plan queda vacío |
| G05 | Arrancar/reiniciar Access y comprobar identidad | Mismo nodo Tailscale e IP esperada; TUN operativo, estado persistente y solo permisos previstos; no re-enrolamiento por una reaplicación |
| G06 | Consultar directamente CoreDNS por UDP y TCP | SOA/NS/A autoritativos; `proxmox.gnx` devuelve IP GNX; nombre desconocido NXDOMAIN; AAAA de nombre existente NOERROR sin datos; zona ajena REFUSED y sin forward |
| G07 | Resolver desde CW y CL sin elegir servidor manualmente | Resolver del sistema y navegador alcanzan `proxmox.gnx`; coincide con G06. Split DNS configurado; GNX no escribe DNS local. Consultar solo con `dig @IP` no aprueba esta fila |
| G08 | Abrir `https://proxmox.gnx` desde CW/CL antes y después de confiar CA | Antes: DNS funciona y el cliente rechaza CA desconocida. Después: certificado válido para el nombre, login Proxmox e interacción/API autenticada; sin bypass TLS ni puertos añadidos a la URL |
| G09 | Verificar Caddy → Proxmox y probar CA/nombre incorrectos en configuración de prueba | Upstream HTTPS verificado; una CA o un nombre TLS incorrectos producen error observable y no fallback inseguro; restaurar confianza recupera la ruta |
| G10 | Solicitar SNI desconocido y Host HTTP desconocido con SNI válido | No se entrega contenido Proxmox ni externo; rechazo TLS o HTTP 4xx según corresponda. Si se prueba wildcard, sigue sin conceder rutas |
| G11 | Probar acceso con identidad autorizada y no autorizada | Solo la autorizada llega a DNS/443; ninguna alcanza 8006, bridge interno, API Caddy o sockets del host. No hay publicación accidental en interfaces LAN/Windows |
| G12 | Publicar X en `external.gnx`, detenerlo, reiniciarlo y retirar ruta | Respuesta testigo correcta; caída externa visible sin romper `proxmox.gnx` ni salud local GNX; vuelve sin reinstalar. Tras retirar ruta, el proxy rechaza aun con DNS en caché |
| G13 | Inyectar zona/Caddyfile inválidos, interrumpir apply y recrear Access | Configuración inválida no sustituye la anterior; siguiente apply reconcilia fase. Al recrear namespace Access, DNS/Control se recuperan; Compute y storage no se recrean |
| G14 | Revisar permisos y usar credenciales canario en el flujo completo | Sin secretos en Git, TOML público, argv, `inspect`, logs, URLs o evidencia; entradas protegidas; ninguna app externa recibe mounts o secretos GNX. Privilegios reales de Dockurr documentados |
| G15 | Invocar desde `gnx.exe` en W y comparar con CLI Linux | Misma operación, intención, estado y códigos de salida; Linux ejecuta la infraestructura. No se escriben hosts/NRPT/adaptadores Windows ni se concede shell remota |
| G16 | Reboot completo de L sin reinstalar ni reaplicar manualmente | systemd recupera las tres capacidades; DNS y HTTPS remotos vuelven; identidad, CA, configuración y storage testigo permanecen |
| G17 | En W, terminar WSL y arrancar; después reiniciar Windows, login y arranque documentado | En ambos casos recupera mediante systemd sin reimportar distro/recrear estado. CW/CL vuelven a usar `proxmox.gnx`. Se registra la intervención y sesión WSL requerida |
| G18 | Conciliar evidencia y repetir smoke final en L y W | G00–G17 completos sobre revisión/digests finales; Proxmox real accesible por HTTPS, externo independiente y ningún defecto abierto que incumpla una fila |

El test de acceso prohibido a 8006 debe hacerse desde clientes remotos; una consulta local de health a la API privada no es exposición pública. Las pruebas de certificados erróneos se realizan en la configuración aislada de laboratorio y dejan la configuración válida restaurada.

En G13, no se exige una transacción distribuida entre CoreDNS y Caddy: se exige orden de publicación seguro, rechazo inmediato en el proxy al retirar host y recuperación de fase. En G04, un proceso sano no debe reiniciarse por mera comparación de timestamps.

## 6. Procedimientos mínimos de verificación

Los comandos GNX siguientes son el contrato a implementar. Los comandos de sistema ilustran cómo observarlo; se guardan los comandos finales exactos en la evidencia.

### Separar DNS directo de resolución del cliente

Desde CL, sustituir `<IP_GNX>` por la IP del registro de ejecución:

```sh
dig @<IP_GNX> gnx SOA
dig @<IP_GNX> proxmox.gnx A
dig +tcp @<IP_GNX> proxmox.gnx A
dig @<IP_GNX> inexistente.gnx A
dig @<IP_GNX> proxmox.gnx AAAA
dig @<IP_GNX> example.com A
getent ahostsv4 proxmox.gnx
```

Repetir **cada tipo** de consulta directa también con `+tcp`, incluyendo las negativas. Guardar rcode, flags de autoridad y respuestas; `+short` por sí solo ocultaría parte de la evidencia. `getent` comprueba el camino de resolución del sistema.

Desde CW, usar el resolver normal y abrir el navegador, sin fijar servidor ni IP:

```powershell
[System.Net.Dns]::GetHostAddresses('proxmox.gnx')
```

Capturar antes y después los ajustes DNS pertinentes, distinguiendo las acciones GNX de la aplicación de políticas por Tailscale. Un cambio legítimo del cliente Tailscale no se atribuye automáticamente a GNX. La documentación oficial advierte que herramientas que consultan un nameserver directamente pueden eludir split DNS del sistema. [Pruebas DNS de Tailscale](https://tailscale.com/docs/reference/dns-in-tailscale#test-dns-configurations).

### Separar TLS de DNS y autenticación

Tras verificar la huella de la raíz pública, en CL:

```sh
curl --cacert /ruta/gnx-root.crt https://proxmox.gnx/
```

Esto verifica la respuesta HTTPS inicial; **no prueba login**. G08 añade sesión autenticada en navegador y consultas API contra el Proxmox esperado. `--cacert` limita la confianza de curl a ese archivo; el navegador requiere su propia confianza configurada y verificada.

Para G10 se puede usar `--resolve` con SNI `proxmox.gnx` y un Host distinto. Es una prueba deliberada del proxy, no evidencia de split DNS. Nunca usar `-k` como aceptación.

### Separar proceso activo de salud y persistencia

Guardar la salida de:

```sh
gnx status --json
gnx doctor --json
systemctl show gnx-access.service gnx-dns.service gnx-control.service gnx-compute.service
```

Completar con identidad, API, almacenamiento y tráfico real. El testigo de storage usa contenido no sensible y checksum conocido; no se comparan hashes de todo un directorio de base de datos que cambia legítimamente durante el arranque.

G17 puede usar `wsl --terminate GNX`; para cubrir `wsl --shutdown` se utiliza un host de laboratorio donde detener todas las distros esté dentro de la prueba. No se ejecuta ese comando sobre otras sesiones del operador incidentalmente.

## 7. Evidencia, fallos y cierre

Cada fila ejecutada produce un registro con gate, perfil, fecha, revisión/digests, precondiciones, pasos exactos, resultado esperado, resultado observado, exit code y enlaces a salidas saneadas. Capturas de navegador prueban interacción, pero no sustituyen los resultados DNS, TLS o API.

La evidencia distingue:

- **Hecho observado:** resultado ejecutado en el entorno indicado.
- **Revisión estática:** inspección de configuración o código.
- **Pendiente/bloqueo:** prueba que aún no pudo ejecutarse y requisito que falta.

No se trasladan resultados de Linux a WSL por similitud. Cambiar imágenes, configuración efectiva o código relevante invalida la evidencia afectada y exige repetir sus gates antes del cierre.

El informe final incluye una tabla G00–G18 × perfil, pasos para repetir el bootstrap, hashes de binarios/configuración e imágenes, confianza TLS pública y límites conocidos. Para éxito, todos los gates obligatorios deben estar en PASS. Un bloqueo termina una sesión de trabajo con evidencia parcial, **no termina el PoC como completo**.

La documentación final debe describir exactamente la experiencia comprobada: **Proxmox de Dockurr bajo Quadlet, persistente y accesible en `https://proxmox.gnx`**, más una ruta externa independiente. No anuncia todavía soporte de workloads LXC/VM ni Windows desatendido.

## 8. Continuidad del proyecto

[arquitectura.md](arquitectura.md) define responsabilidades y este documento define aceptación. Son la referencia de la propuesta actual.

Los planes o prompts antiguos de `.AGENTS/` que todavía exigen dos mallas, instaladores finales, adaptación de WatchTower o backups de workloads están desfasados. Antes de una campaña de implementación deben alinearse con estos gates, preservar el progreso real y respetar la instrucción vigente de trabajar sin subagentes. Actualizar documentación no autoriza iniciar esa campaña ni acredita pruebas de producto.
