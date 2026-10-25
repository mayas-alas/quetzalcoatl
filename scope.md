# GnX — qué sí, qué no y cómo decidir

**Estado: diseño 0.6; implementación acotada autorizada el 2026-10-25.** No es un backlog abierto.

**Instrucción vigente (2026-10-25):** bloque de 23 minutos, Rust + configuración
externa + scripts mínimos; commits continuos con `Hector <hector@email.gnx>`.
La fase documental descrita abajo es histórica: ahora se permite implementar y
compilar, sin convertir decisiones abiertas en aprobadas ni compilar en aceptar.

**Convivencia D11 para este bloque:** dejar el legado intacto, sin limpieza ni
adopción, por instrucción posterior del usuario. Nueva identidad: servicio
`GnXCore`, pipe `GnX.Core.v1`, cuenta `gnxcoresvc`, distro `gnx-core`, estado en
`%ProgramData%\GnX Core`. Este bloque no instala ni crea cuentas en el host.
La desinstalación legacy fallida se conserva como incidencia separada.

**Pruebas, instrucción vigente:** no generar código de tests, fixtures, mocks,
harnesses ni infraestructura de pruebas por ahora. No crear escenarios ni
simulaciones, tampoco como scripts externos. Su paradigma y ejecución quedan
fuera de este esfuerzo. Formato/compilación no son pruebas de aceptación;
diagnóstico sobre estado real mediante `systemctl`/`journalctl` y SCM/Visor de
eventos. No crear un observador/logger propio ni desplegar entornos de prueba.
No omitir validaciones de seguridad ni afirmar aceptación real. Agentes que retomen deben
mantener esta restricción hasta nueva instrucción; prevalece sobre las propuestas
históricas de tests de este documento.
Objetivo: un MSI/EXE instala el núcleo que entrega `https://app.gnx` y
`https://compute.gnx` desde WSL del usuario dedicado, operado por `gnx`.
Destino final: app recibe archivos o la CLI un path de Quadlets/configuración;
`Containerfile`/contexto solo si hace falta build. Primer caso real confirmado:
un LXC con RustFS, Pocket ID y NetBird; Pocket autentica solo a NetBird y RustFS.

## 1. Autoridad y fase

```text
instrucción actual del usuario
           |
           v
acuerdos explícitos en este marco ---> architecture.md
           ^                                 |
           |                                 v
     resolver preguntas                 implementación futura

legacy ---> evidencia para preguntar; NUNCA autoridad ni permiso para portar
```

- **C** = confirmado por la petición actual; **P** = propuesta, no aprobada;
  **A** = pregunta abierta. No convertir P/A en C por silencio o conveniencia.
- Fase actual: **implementación Rust/configuración, en `main`, un agente**.
  Los tres documentos conservan alcance y criterios; no equivalen a aceptación.
  La instrucción actual autoriza código/build, no instalación, migración ni
  modificación del runtime antiguo. No escribir código de tests ni escenarios.
- Resolver solo las decisiones de las que dependa la siguiente operación.
- Si los documentos discrepan, detener la parte afectada y resolver; no escoger
  la interpretación que permita hacer más trabajo.

## 2. Sí confirmado / no incluido

| ID | Sí queremos (C) | No se deriva de ello |
| --- | --- | --- |
| C01 | `gnx` como CLI Windows pequeña y no verbosa. | TUI, WebView, GUI de setup, framework frontend o comandos ilimitados. |
| C02 | Código propio ejecutable Rust; configuración externa y `.run`/`.sh` permitidos. | Scripts/templates/configuración incrustados en Rust; `.in`; segundo backend propio en otro lenguaje. |
| C03 | Usuario Windows dedicado con su perfil y WSL propios. | Usar WSL/SSH/home/credenciales del usuario host; adoptar cuentas por nombre. |
| C04 | Pipes propios para control e información sensible. | HTTP localhost de control, keys por navegador, argv, URL, logs o Git. |
| C05 | Servicio Windows: resiliencia y puente host ↔ identidad ↔ WSL. | CLI permanente, tarea de login como sustituto, shell remoto o plataforma de plugins. |
| C06 | systemd + Podman/Quadlets, incluido PVE; lifecycle nativo sin capa propia de workloads. | Docker Desktop obligatorio; APIs `workloads start/stop`, supervisor duplicado o scripts infinitos de reinicio. |
| C07 | Núcleo: `app.gnx` y Proxmox en `compute.gnx`. El caso C15 tendrá acceso propio acordado por el gateway existente. | Dominios/puertos/IdP nuevos en el núcleo por herencia o sin necesidad aprobada. |
| C08 | Instalador MSI y EXE; WiX o alternativa FOSS realmente mínima. | Instalador artesanal, migrar de toolchain por gusto o depender de .NET en runtime. |
| C09 | Alcance estricto, documentación puntual y diagnóstico nativo. Tests/escenarios fuera de este esfuerzo por instrucción vigente. | Omitir seguridad, inventar escenarios o crear infraestructura de tests/observabilidad. |
| C10 | Aprender del legacy, preservándolo. | Restaurarlo entero, heredar sus mandatos o rehacer su historia. |
| C11 | WSL base: **Tailscale, PVE, Caddy y Pi-hole**, bajo systemd/Quadlets. | Sustituir Pi-hole por otro DNS por preferencia; añadir servicios de negocio al nodo. |
| C12 | App entrega Quadlets/archivos; CLI admite path local. Mismo despliegue a LXC de PVE; `Containerfile`/contexto solo si necesita build. | Dos motores por tener dos entradas; build obligatorio; path de navegador tratado como acceso al host; ejecución SYSTEM/root sin límites. |
| C13 | MSI/EXE autocontenidos: filesystems preparados, runtime/dependencias, cuatro OCI base y configuración/Quadlets; base LXC e inputs del caso C15 locales para el alcance offline declarado. | Descargar lo faltante al instalar; clonar instancia enrolada; caché universal para cualquier aplicación; provisionar apps por defecto. |
| C14 | Caddy y Pi-hole operan localmente; app muestra avance antes/durante el enrolamiento. El usuario introduce la key por `gnx` y el sistema la entrega al Tailscale de su WSL dedicada. | Exigir Internet, IP mesh, key o PVE listo para mostrar app; recopilar la key en web; incrustarla en la imagen distribuida. |
| C15 | Primer despliegue real por C12: **un LXC, RustFS + Pocket ID + NetBird**, gestionados por Quadlets; Pocket integrado solo con las otras dos aplicaciones. | Restaurar private-services legacy, añadir auth a app/PVE/Tailscale, reemplazar la mesh del nodo o crear otro supervisor. Tres apps no implica ocultar dependencias: D14. |
| C16 | Avanzar por hitos de hasta 20 minutos, entregar evidencia/resultado y continuar sin bloqueos globales. | Garantizar todo instalado/login humano en 20 minutos; saltar seguridad, declarar éxito falso o parar trabajo independiente por una operación pendiente. |
| C17 | `gnx start` y `gnx stop` desde CLI para iniciar/detener el nodo propio, delegando en systemd y respetando la parada explícita. | Subcomandos `workload/workloads`, selección de aplicaciones, otro supervisor, apagado global de WSL o borrado de datos/identidad. |

**Excluido:** Continuity, AI/Voice/Oryx/OmniRoute, GitHub Actions, plataforma de
publicación S3, importador de perfiles legacy, catálogo de workloads, motor
genérico de pipelines, OAuth/IdP para app.gnx, PWA
offline/service worker, telemetría remota, auto-updater y multiusuario/multinodo.
No añadirlos como "preparación", dependencia inventada, placeholder ni demo.
Un LXC de aplicación tampoco es parte de la instalación base.

**No confundir:** RustFS/Pocket ID/NetBird entran en C15 como servicios desplegados;
no como dependencias del núcleo. Llevar sus imágenes offline no crea instancias.
Separa tres hitos, no tres productos ni tres motores:

```text
AHORA                 PRIMER NÚCLEO                  DESTINO FINAL
3 documentos   -->    instalador + CLI + WSL    -->  app/CLI: Quadlets/contexto
                      4 servicios + 2 HTTPS          -> 1 LXC, 3 aplicaciones
                      sin apps activadas             Pocket -> NetBird/RustFS
```

Las imágenes de terceros conservan su stack: no significa reescribir Proxmox,
Tailscale o systemd en Rust. La interpretación propuesta del frontend está en D01.

## 3. Límites de seguridad y propiedad propuestos (P)

Estas reglas hacen viable el objetivo sin expandir funcionalidades. Revisarlas
como contrato, no como lista de features.

```text
RECURSO EXISTENTE
     |
     +-- ownership verificable --> conservar / reconciliar según operación
     |
     +-- ajeno o incierto ------> informar conflicto; NO adoptar ni eliminar

falló health != permiso de reset
falló setup  != permiso de limpiar
coincide nombre != ownership
```

| Área | Sí | No |
| --- | --- | --- |
| Identidad | SID/ownership y perfil comprobados; mínimos privilegios. | Cuenta dedicada administradora por comodidad, contraseñas hardcoded. |
| IPC | Cliente y servidor autenticados, operaciones acotadas, límites y timeouts. | Shell/rutas/comandos libres; confiar solo en el nombre del pipe. |
| Quadlet/contexto | Mismo contrato en app/CLI; validar privilegios/mounts/entradas, build aislado solo si hace falta. | Tratar Quadlets o `Containerfile` como datos inocuos; montar sockets/pipes/secretos del nodo dentro del build. |
| Secretos | Entrada sin eco, permisos antes de escribir, limpieza y persistencia justificada. | Imprimir, pedir que se peguen en chat, inspeccionar env/archivos con valores. |
| Privilegios | Diferenciar servicio SYSTEM, proceso Windows dedicado y root Linux. | Suponer que cambiar UID Linux cambia el propietario Windows de la WSL. |
| Proxmox | Autenticación nativa y TLS backend verificado. | Bypass de login/TLS/licencia o repetir una imagen antigua sin inspección. |
| Red | Loopback/mesh autorizada; ownership de reglas/nombres/certificados. | Exposición LAN/Internet, DNS abierto, alterar políticas externas sin permiso. |
| Resiliencia | SCM → servicio; systemd/Quadlets → contenedores; PVE → LXC. Intención mínima de nodo y stop respetado. | Duplicar el lifecycle con un framework de workloads; prometer memoria liberada o disponibilidad no medidas. |
| Actualizar | Diferenciar paquete instalado de runtime aplicado; preservar estado. | Pull/restart en `status`, auto-upgrade, downgrade/reset oculto. |
| Retirar | Plan explícito, alcance propio y política de conservación acordada. | `wsl --shutdown`, prune global, unregister/purga sin autorización o borrar respaldos. |
| Host | Detectar requisitos y explicar el bloqueo. | Tocar BIOS, TPM, Secure Boot, BitLocker, antivirus, parches o reiniciar por sorpresa. |

## 4. Preguntas respondidas con evidencia legacy

Referencia fija **L = `legacy/v0.1.93-2026-10-13` @ `1786627`**.
Consulta sin cambiar de rama: `git show L:ruta` sustituyendo L por la referencia.
El snapshot antiguo **H = `0f28352` (v0.1.52)** no es el mismo producto que L.

| Pregunta | Respuesta verificable | Consecuencia para el nuevo diseño |
| --- | --- | --- |
| ¿Debemos respaldar otra copia del árbol antes de empezar? | L conserva el producto; `main` @ `e038a5e` arrancó con reglas Git. | Referenciar commits; no duplicar fuentes, ejecutables ni secretos en main. |
| ¿El legacy era solo el núcleo? | L:`docs/architecture.md`, `docs/config-driven-platform.md` añaden auth, workloads y migraciones. | Sus dependencias no son requisitos de esta entrega. |
| ¿Existía la idea de carpeta/bundle hacia LXC? | L:`workload_bundle.rs` y `payload/node/workloads/deploy.run` tienen admisión de perfiles; `docs/config-driven-platform.md` declara objetivos pendientes. | Evidencia para C12, no una implementación aceptada de carpeta/Containerfile genérica. No copiar sus workloads ni dependencias. |
| ¿Proxmox se construía en este repo? | L:`payload/node/services/compute.container` consume `ghcr.io/mayas-alas/tailnet-proxmox` por digest; build externo. | Elegir imagen en D05; no abrir aquí otro proyecto de imagen. |
| ¿KVM es imprescindible para la web y LXC? | No por su función; H:`compute/lxc.run` usa `pct`. H obliga KVM en install/verify/Quadlet. | Requisito de aquel paquete, no necesidad universal del núcleo. Resolver D02. |
| ¿Basta con `REQUIRE_KVM=N` en cualquier imagen? | Solo se comprobó en el entrypoint del digest Dockurr de H, revisión upstream `e18ddeb8461b09a2c2e8a9d64ada1272c1442109`. L usa otro OCI. | Inspeccionar el build/entrypoint exactos de D05; no dar por validado el modo LXC. |
| ¿Un proceso bajo SYSTEM usa la WSL dedicada automáticamente? | L:`dedicated_account.rs` necesita token, perfil, entorno y `CreateProcessAsUserW`. | Probar la frontera Windows real; no usar `wsl -u root` como sustituto. |
| ¿Hay que copiar los bundles temporales de secretos? | L:`secret_store.rs` y `control_pipe.rs` los usan; es una implementación, no una exigencia. | Proponer pipe directo acotado en memoria; persistir solo lo necesario. |
| ¿MSI instalado equivale a servicios listos? | L:`installer/README.md` separa MSI de provisioning Linux. | Verificar URLs al final; no declarar listo por exit 0 del instalador. |
| ¿Había sistema WSL empaquetado para importar offline? | `gnx-node-rootfs.tar` aparece desde `57e6287` (2026-09-24), 37.744.448 bytes con SHA-256; H/L lo empaquetan. H:`install.run` todavía usa apt/dnf para dependencias. | Rootfs local no acredita instalación completa offline. D10 debe cubrir runtime, cuatro imágenes y prerrequisitos Windows. |
| ¿"Fresh install" era inocuo? | L:`installer/Product.wxs` ejecuta `CleanupFreshResidue`; uninstall también puede purgar. | No portar esas acciones; acordar D11 antes de usar el host existente. |
| ¿Stop era siempre local a GnX? | L:`installer/README.md` describe un stop con `wsl --shutdown` global. | Rechazar esa herencia; detener únicamente recursos administrados. |
| ¿Lo archivado acredita una release aceptada? | L:`docs/legacy-snapshot.md` avisa que fuente, EXE y validación no están equiparados. | Fuente/hashes/build/instalación/salud son evidencias distintas. |

Rutas abreviadas en la tabla: `compute/lxc.run` = `payload/node/compute/lxc.run`;
archivos Rust = `apps/host-agent/src/`. No afirmar que el legacy esté validado
ni que el host actual sea una instalación limpia.

### Research aplicado: instalación local ≠ enrolamiento mesh

Consulta documental del 2026-10-24; **no es validación live del nuevo producto**.
Preguntas contrastadas antes de corregir la arquitectura:

| Pregunta propia | Evidencia y respuesta | Regla resultante |
| --- | --- | --- |
| ¿Debo preguntar si app puede arrancar antes de mesh? | El usuario lo requiere (C14); Caddy soporta TLS local sin ACME [R3]. | No reabrirlo: preparar acceso local antes del prompt mesh. |
| ¿Por qué el legacy no daba ese recorrido? | H:`payload/node/install.run` espera auth y detiene DNS/gateway/compute si falta; L:`services/dns.container` y `gateway.container` requieren mesh. | No portar el gate ni esas dependencias a la vista local. |
| ¿Pi-hole necesita Internet o `tailscale0` para DNS local? | No: registros locales y upstreams son configuraciones distintas [R4]. L los ataba a interfaz/IP mesh. | Bootstrap local independiente; agregar acceso mesh después, sin DNS abierto ni bucle resolver. |
| ¿Caddy dentro de WSL instala confianza automáticamente en Windows? | No se puede asumir: Caddy advierte límites de instalación de trust en contenedores [R3]. H:`host_access.rs` era la frontera Windows. | Publicar confianza/nombres/forward local antes de declarar app accesible, no esperar NodeReady remoto. |
| ¿Se puede mostrar avance sin un servidor nuevo? | H:`gateway/routes.conf` sirve app y `/status.json`; H:`linux_node.rs::publish_app_status` publica una proyección. | Reutilizar la idea de archivo sanitizado/atómico y Caddy; no heredar el gate de readiness global. |
| ¿La key debe quedar en argv o en el instalable? | No: la CLI Tailscale admite `--auth-key=file:...` [R1]; el valor puede llegar por pipe/archivo protegido en runtime. | Credencial del usuario solo en la instancia dedicada; comprobar el contrato de la versión fijada. |
| ¿Key de alta e identidad persistente son lo mismo? | No: `TS_STATE_DIR` conserva el peer; `TS_AUTH_ONCE` evita reautenticación innecesaria [R2]. | Conservar estado del peer; no exigir otra key por cada restart ni copiar identidades entre instalaciones. |
| ¿Un fallo de enrolamiento vuelve fallida la instalación offline? | No: son operaciones distintas; C14 conserva observabilidad local. | App sigue mostrando pendiente/error/acción; no purgar ni parar la base por key inválida o red ausente. |

Fuentes oficiales consultadas (el digest/versionado futuro aún debe verificarse):
- **R1:** [Tailscale up: auth key por archivo y timeout](https://tailscale.com/docs/reference/tailscale-cli/up).
- **R2:** [Contenedor Tailscale: estado persistente y auth once](https://tailscale.com/docs/features/containers/docker/docker-params).
- **R3:** [Caddy: HTTPS local y confianza en contenedores](https://caddyserver.com/docs/automatic-https#local-https).
- **R4:** [Pi-hole: registros, upstreams e interfaces](https://docs.pi-hole.net/ftldns/configfile/).

En esta tabla, `services/*` y `gateway/*` son relativos a `payload/node/`;
los archivos Rust a `apps/host-agent/src/`. La retención reusable de la key sigue
siendo D06; no confundirla con persistir el estado del peer ni con C14.

### Research aplicado: caso LXC sin redundancia

| Pregunta | Respuesta documental | Consecuencia |
| --- | --- | --- |
| ¿El legacy eran solo tres contenedores? | L:`workloads/private-services/` contiene RustFS, Pocket, NetBird server y dashboard, además de sidecar mesh. | No copiar ese stack; un LXC y tres aplicaciones requeridas, contador OCI pendiente D14. |
| ¿NetBird combined incluye su dashboard? | No: [referencia oficial](https://docs.netbird.io/selfhosted/configuration-files) separa server y dashboard. | Si se quiere UI, admitir explícitamente cuarta unidad o acordar administración sin ella; no fusionar procesos para ocultarla. |
| ¿Pocket → NetBird exige otro proxy de autenticación? | No: [guía oficial PocketID](https://docs.netbird.io/selfhosted/identity-providers/pocketid) soporta integración nativa; hay API de proveedores. | Config/API soportada primero. No añadir OAuth2 Proxy, Valkey o un IdP externo adicional. |
| ¿RustFS admite Pocket por configuración? | [OIDC de RustFS](https://docs.rustfs.com/en/security-compliance/oidc) exige discovery, cliente confidencial, PKCE, callbacks y claims/políticas. [Pocket](https://pocket-id.org/docs/setup/connect-an-app) admite clientes OIDC. | Viable por contrato; fijar versiones y comprobar login/permisos reales. No afirmar compatibilidad del viejo digest ni confundir SSO Console con claves S3. |
| ¿La GUI elimina todos los requisitos humanos? | No: passkey/consentimiento puede requerir presencia del usuario. | API/config primero; si falta vía soportada, skill GUI adecuada. Registrar pendiente sin bloquear núcleo ni simular interacción personal. |

No desplegado ni aceptado todavía. Sin nueva mesh por guest ni otro Caddy/Pi-hole;
el enrutamiento al LXC y STUN UDP se acuerdan y verifican, no se deducen de HTTPS.

## 5. Decisiones que debemos cerrar juntos (A)

Las filas abiertas ofrecen una propuesta **P**, no una respuesta del usuario.
D04 ya quedó resuelta por la instrucción actual; no reabrirla por preferencia.

| ID | Pregunta concreta | Propuesta mínima (P) / consecuencia |
| --- | --- | --- |
| D01 | C14 confirma avance local; C12 confirma entrada Quadlet/contexto por app/CLI. ¿Quién accede a la vista y cómo se presenta el avance? | Propuesta: archivos estáticos + estado sanitizado servido por Caddy, sin otro backend de observación. No volver a preguntar si mesh debe bloquear app: no. Autorización de mutaciones finales en D12. |
| D02 | ¿Compute inicial es LXC sin VMs aceleradas? | Sí como propuesta: KVM no bloquea el núcleo; VM acelerada no prometida. No crear workloads para demostrarlo sin permiso. |
| D03 | C14 confirma acceso desde el host sin mesh. ¿Qué clientes tailnet externos entran en la aceptación? | Acordar solo el alcance remoto: ACL, Split DNS y confianza CA del operador. La conectividad remota no condiciona el acceso/avance local confirmado. |
| D04 — C | ¿Qué DNS entra? **Pi-hole**, elegido por el usuario. | Configurarlo para el namespace requerido y sin resolver abierto. No añadir filtros, productos ni flujos de administración innecesarios. |
| D05 | ¿Cuál es la imagen Proxmox autorizada y cómo recibe su contraseña? | Evaluar primero la imagen propia de L; fijar fuente/revisión/digest y revisar arranque LXC sin KVM, licencia y secretos. Sin asumir compatibilidad con Dockurr H. |
| D06 | ¿Key Tailscale de un solo uso o reusable almacenada? | Consumir y eliminar; conservar estado del peer para reinicios. Reusable solo si hay necesidad explícita, custodia y política de borrado. |
| D07 | ¿Qué Windows/CPU, bases WSL/LXC y versiones soportamos? | Un baseline Windows 11 x64 + WSL2; fijar bases, Podman, dependencias y RAM/disco medidos. C13 obliga inputs locales; no descargas implícitas ni versiones/límites arbitrarios del legacy. |
| D08 | ¿`gnx status` necesita elevación? | Un solo pipe admin en v1 como opción mínima. Lectura no elevada requiere permiso y esquema público limitados, no acceso al control privilegiado. |
| D09 | ¿Cómo publica el servicio el HTTPS local? | Forward TCP loopback → WSL sin terminar TLS; elegir Rust o mecanismo Windows por evidencia. Revisar puerto 443 ocupado, cambio IP y ownership; no dos proxies competidores. |
| D10 | C13 confirma núcleo offline completo. ¿WiX/Burn es aceptado y qué prerrequisitos Windows se incluyen? | MSI completo + Burn EXE que lo contiene, todos los inputs del producto locales. Verificar licencia/condiciones FOSS y redistribución WSL; .NET solo build. Fijar baseline del host y medir tamaño. No confundir instalación offline con enrolamiento Tailscale o build arbitrario sin conectividad. |
| D11 | ¿Cómo convive la nueva entrega con GnX ya instalado y qué conserva uninstall? | No tocarlo hasta plan explícito. Propuesta: preservar datos por defecto y separar purga confirmada; fijar identidad del producto/UpgradeCodes, ownership y recuperación antes del primer MSI. |
| D12 | App y CLI son entradas confirmadas. ¿Cuál es el transporte/autorización de la web al mismo despliegue? | El navegador no abre Named Pipe ni manda un path con acceso implícito al host. Resolver puente nativo o receptor Rust del mismo origen, no ambos. No añadir IdP a app ni otro contenedor base por anticipación. |
| D13 | C15 fija un LXC compartido. ¿Cómo se crea/selecciona, conecta y publica? | PVE + Quadlets nativos; fijar recursos/datos, límites de admisión/build y ruta nodo↔LXC sin sidecar mesh. Build opcional. Caddy/Pi-hole existentes; no escoger VM, registry ni LXC por servicio por defecto. |
| D14 | ¿Tres aplicaciones con NetBird administrado por API o también dashboard? ¿Qué versiones/orígenes OIDC? | Dashboard oficial separado implica cuarto contenedor. Fijar imágenes, issuer/callbacks/nombres y roles; dos clientes Pocket separados. No abrir red pública ni integrar Pocket con app/PVE/Tailscale. No frenar el núcleo por esta elección del despliegue. |

**Orden de conversación:** D01–D03 y D12–D13 fijan la experiencia/destino final;
D04–D07 sus dependencias; D08–D11 su operación/entrega; D14 el caso OIDC.
Lo pendiente detiene solo su operación dependiente, no revisión ni trabajo
independiente autorizado. Una validación de seguridad fallida no se omite.

## 6. Puerta de decisión antes de actuar

```text
petición -> ¿objetivo C y operación autorizada?
                     | no/ambiguo -> investigar/preguntar SOLO por esa operación
                     v
          ¿decisiones D necesarias resueltas?
                     | no -> resolver solo la dependiente
                     v
          ¿recursos propios + secretos + privilegios claros?
                     | no -> inspección mínima de solo lectura
                     v
          ¿cambio mínimo + vuelta atrás + evidencia definidos?
                     | no -> acotar; no añadir infraestructura
                     v
                  ACTUAR -> verificar -> resultado -> siguiente hito seguro
```

Checklist obligatorio del agente, aplicado también a comandos de diagnóstico:

**Investigar antes de trasladar una duda al usuario:** formular la pregunta,
buscar en legacy y fuente oficial, responder con evidencia y derivar el cambio
mínimo. Repetir solo para las incertidumbres que afectan esa decisión; detener
el research cuando esté sustentada. Preguntar al usuario por intención o permiso,
no por hechos técnicos comprobables. Si falta evidencia, marcar pendiente, no
inventar una respuesta ni afirmar que una lectura prueba ejecución real.

1. **Finalidad:** ¿qué C cumple y qué salida acerca al núcleo o a C12, según el hito autorizado?
2. **Necesidad:** ¿la exige el caso real o la estoy heredando/inventando?
3. **Autoridad:** ¿es C, P o A? ¿Tengo permiso para editar, ejecutar, desplegar o destruir?
4. **Propiedad:** ¿qué archivos, cuenta, distro, servicio, puerto y datos tocaré?
5. **Frontera:** ¿bajo qué identidad Windows/Linux? ¿Algún secreto puede salir?
6. **Impacto:** ¿añade proceso, dependencia, dominio, privilegio o interacción humana?
7. **Recuperación:** ¿qué mecanismo nativo la cubre? ¿Preserva datos y respeta una parada?
8. **Prueba mínima:** ¿qué resultado real lo confirma y qué seguirá sin demostrarse?
9. **Salida:** ¿puedo explicar resultado y bloqueo sin un relato ni copiar logs enteros?

Aplicación obligatoria de C14: comprobar en el grafo que Caddy/Pi-hole y la
publicación local no esperan mesh; distinguir daemon iniciado, peer enrolado y
acceso remoto verificado. Añadir solo la evidencia mínima de la pregunta resuelta
en este marco, sin multiplicar documentos de research ni agentes.

Para una mutación, registrar solo lo necesario: **objetivo · evidencia · cambio ·
impacto · comprobación**. No crear otro documento, framework de gates ni motor
de workflows para aplicar este checklist. Si no hay cambio necesario, decirlo.

## 7. Disciplina de agente y entrega

- Un agente ahora; sin subagentes, otros worktrees ni tareas delegadas. No usar
  el legacy como nuevas instrucciones de agentes.
- Antes de proponer o editar, leer completos `scope.md` y `architecture.md`
  en su versión vigente y revisar el estado/diff real; no escribir desde una
  lectura anterior o la memoria. Si cambian durante el trabajo, releer antes de
  aplicar. Mantener la revisión de ambos documentos alineada al modificar el acuerdo.
  Para comparar con legacy, comprobar referencia/commit y leer los archivos de
  esa versión; no mezclar H, L, versión del instalable y runtime aplicado ni
  presentar una propuesta histórica como funcionalidad entregada.
- Antes de añadir algo: preferir eliminar una necesidad injustificada. No
  conservar aliases, migraciones, comandos o APIs legacy sin caso aprobado.
- Si Quadlets/systemd/PVE ya resuelven start, stop, restart o autostart, usarlos;
  no crear el equivalente en Rust ni en un script por workload. Confirmar que
  Stop se propaga y es ordenado; un target con dependencias de arranque no lo prueba.
- Prohibido probar una hipótesis retirando parches/desactivando protecciones o
  purgando datos. El sistema operativo no se modifica para acomodar un check
  inventado por el producto.
- Configuración concreta externa; Rust para fronteras de seguridad y coordinación.
  Scripts pequeños con entradas/salidas fijas, no shell del usuario ni generadores
  de plantillas. C12 admite Quadlets y build aislado opcional, no scripts confiables
  del nodo. Sin Node/Python/Go propios como segunda aplicación.
- Validación futura proporcional: unas pruebas de autorización IPC y secretos,
  instalación real, ambas URLs, stop/start y recuperación sin pérdida. Sin
  framework E2E, mocks masivos, benchmarks, workloads demo ni pantallas como única prueba.
  Para C17: comprobar `gnx stop`, parada respetada tras recuperación del servicio,
  `status` sin despertar el nodo y `gnx start` sin reenrolar ni actualizar;
  conservar datos y no detener WSL ajenas. Contrato en architecture.md, sección 6.
  Al implementar C12/C15, comprobar el caso autorizado en LXC, datos y login OIDC
  NetBird/RustFS; no activar esas aplicaciones como dependencias del núcleo.
  Para C14: app accesible antes de key, avance visible al enrolar y app conservada
  con key inválida/red ausente; un fallo mesh no reinicia Caddy/Pi-hole.
- No confundir `propuesto`, `compilado`, `instalado`, `aplicado`, `sano` y `aceptado`.
  No declarar FOSS, compatibilidad, restart seguro o HTTPS sin evidencia.
- Comunicación breve: resultado, evidencia mínima, límite y decisión necesaria.
  No narrar lecturas rutinarias ni copiar detalles sensibles. Diagnóstico ampliado
  solo cuando ayude a una decisión.
- Git: cambios pequeños, sin reescribir historia ni mover ramas legacy. No subir
  binarios, secretos o evidencias voluminosas a main. Commit/push y publicación
  son acciones distintas; no se presumen por aprobar el diseño.

**Bloque de 20 minutos:** elegir un hito autorizado y verificable, usar primero
capacidades nativas y cerrar con resultado/evidencia, qué queda pendiente y el
siguiente paso exacto. Si algo depende de red, passkey o una decisión, conservar
el estado y continuar trabajo independiente seguro. No esperar indefinidamente,
reiniciar desde cero ni declarar entrega completa por agotar el tiempo. Reanudar
leyendo estos dos documentos y el estado/diff real, no de la memoria del agente.

## 8. Cierre del primer bloque de implementación

```text
HOY                           PENDIENTE
CLI/servicio Rust compilados   instalador + inputs offline + preparación
target y drop-ins externos     Quadlets completos + HTTPS local
legacy intacto                despliegue LXC + OIDC real
```

Revisar C/P/A juntos, resolver las decisiones que cambien la arquitectura y
quitar las opciones descartadas. Entonces `architecture.md` será el plano
único; este marco conservará las exclusiones y la puerta de decisión.
[Historias de usuario](historias-de-usuario.md) expresa el recorrido y la
aceptación derivados, sin convertir P/A en C ni ampliar implementación. La
compilación no acredita que `app.gnx`/`compute.gnx` ya funcionen. El ejecutable
actual es una entrega parcial, no el instalador ni el producto terminado.
