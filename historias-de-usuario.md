# GnX — historias de usuario y visión 360°

**Estado: borrador 0.1 para revisar, 2026-10-24.** Basado en `scope.md` y
`architecture.md`, borradores 0.6. No acredita implementación ni aceptación.

**Actualización 2026-10-25:** la instrucción vigente de `scope.md` autoriza un
primer bloque de implementación con identidades nuevas y sin tocar el legado.
Las menciones a fase documental más abajo son históricas; ninguna historia se
marca aceptada por esta autorización ni por una compilación.
No generar tests, escenarios ni simulaciones: diagnóstico nativo con
`systemctl`/`journalctl` y SCM/Visor de eventos, sin infraestructura propia.

## 1. Para qué sirve este documento

Entender el producto desde el recorrido del usuario: qué obtiene, qué puede
fallar, qué queda fuera y qué evidencia permitiría aceptar la entrega.
No es otro backlog, un plan de sprints ni una orden para construirlo todo.
«360°» significa cubrir el recorrido y sus límites, no inventar funcionalidades
ni imponer una cantidad de historias.

| Documento | Responsabilidad |
| --- | --- |
| [scope.md](scope.md) | Autoridad de alcance: requisitos Cxx, propuestas P, preguntas Dxx y exclusiones. |
| [architecture.md](architecture.md) | Diseño técnico dentro de ese alcance; los mecanismos propuestos siguen sujetos a decisión. |
| Este documento | Historias y criterios derivados para conversar y verificar el resultado. No amplía los otros dos. |

Antes de usar una historia, leer los documentos rectores vigentes y comprobar
sus revisiones. Si cambian, revisar la historia afectada. Una discrepancia se
resuelve antes de implementar; no se escoge el texto que permita hacer más.

**C** identifica un objetivo ya confirmado; **P**, una propuesta no aprobada;
**A**, una decisión abierta. Los criterios siguientes son una formulación para
revisión de los contratos existentes: no aprueban por sí mismos nuevos detalles.
Que una historia tenga base C no resuelve sus Dxx ni autoriza ejecutarla ahora.

## 2. Producto, persona y valor

**Como persona que opera su propio nodo Windows, quiero instalar GnX, acceder
a su estado local y a Proxmox, y desplegar servicios mediante Quadlets en LXC,
para usar mis aplicaciones sin mantener una terminal abierta ni perder mi
identidad o datos al detener y reanudar el núcleo.**

- **Persona usuaria/operadora:** consulta la app, utiliza la CLI y entrega archivos.
- **Persona administradora:** la misma persona cuando autoriza operaciones
  privilegiadas, configuración o instalación. No son cuentas de producto ni
  un requisito multiusuario/RBAC.
- Navegador, servicio Windows, WSL, PVE y aplicaciones son componentes, no
  nuevos actores humanos. Pocket ID no autentica la app ni la CLI.

El valor no es disponer de otro supervisor o catálogo: es obtener acceso y un
despliegue verificable usando los mecanismos nativos ya acordados.

## 3. Mapa del recorrido

```text
AHORA: revisar alcance + arquitectura + historias; cero cambios de producto/host

DESPUÉS, SOLO CON AUTORIZACIÓN DE IMPLEMENTACIÓN:

instalar paquete autocontenido
        -> identidad/WSL propias + servicios locales
        -> app.gnx con HTTPS y avance, antes de la key mesh
        -> introducir key por CLI -> Tailscale enrolado si hay red/credencial válida
        -> compute.gnx disponible según sus propios requisitos
        -> entregar archivos desde app O path desde CLI
        -> misma admisión -> build solo si hace falta -> LXC + Quadlets
        -> RustFS + Pocket ID + NetBird -> comprobar acceso/OIDC/datos

operación cotidiana: status / stop / start del nodo
mantenimiento: paquete instalado != runtime aplicado; conservar estado
```

Tres resultados distintos, no un «todo listo» global:

1. **App local disponible:** puede mostrar que mesh o compute siguen pendientes.
2. **Núcleo aceptado:** instalación offline, ambas URLs y operación verificadas
   conforme al alcance de aceptación acordado.
3. **Destino final aceptado:** entrega app/CLI y caso real LXC verificados.

El acceso remoto por tailnet tiene su propia decisión D03; no bloquea la app
local. Un paquete offline no permite enrolar Tailscale sin acceso a su control
plane ni construir cualquier aplicación sin sus dependencias locales.

## 4. Historias y criterios de aceptación

### A. Obtener y preparar el núcleo

#### HU01 — Instalar desde MSI o EXE
**Como administradora, quiero un instalable autocontenido para preparar el nodo
sin descargar piezas del producto durante la instalación.**

Base **C08/C13**. Pendientes **D07/D10/D11**.
- MSI y EXE contienen los inputs acordados para el baseline Windows soportado.
- La instalación offline no ejecuta descargas implícitas; un input faltante o
  inválido se informa, no se sustituye silenciosamente desde Internet.
- No crea el LXC de aplicaciones por defecto ni declara salud por el exit 0 del MSI.

#### HU02 — Operar con una CLI pequeña
**Como operadora, quiero usar `gnx` para obtener resultados y acciones claras
sin otra interfaz de setup ni una consola que deba quedar abierta.**

Base **C01/C02/C05**.
- La salida habitual es breve; el progreso se comunica por cambios de fase.
- No incorpora TUI, WebView ni credenciales en argumentos.
- Los ejecutables propios son Rust; configuración y scripts permitidos permanecen externos.

#### HU03 — Tener identidad y WSL propias del nodo
**Como administradora, quiero separar el runtime GnX de mi entorno personal
para no reutilizar mis distros, perfiles o credenciales.**

Base **C03/C05**. Diseño de ownership/permisos **P**; convivencia **D11**.
- Cuenta, perfil y registro WSL pertenecen a la identidad dedicada comprobada.
- Cambiar el UID Linux no se presenta como prueba de identidad Windows.
- Una cuenta/distro de nombre coincidente no se adopta ni elimina por coincidencia.

#### HU04 — Autorizar operaciones sin exponer secretos
**Como administradora, quiero que la CLI entregue el control y los secretos por
pipes propios para no convertir el navegador en una consola privilegiada.**

Base **C04/C05**. Mecanismos de autorización **P**, lectura **D08**.
- Solo clientes autorizados acceden a operaciones acotadas; no se admite shell libre.
- Keys y contraseñas no aparecen en URL, argv, logs, estado público ni Git.
- La sesión/página web no concede permisos administrativos Windows.

### B. Acceder localmente y enrolar la red

#### HU05 — Abrir la app antes del enrolamiento
**Como operadora, quiero abrir `https://app.gnx` antes de introducir la key
Tailscale para conocer el avance local aunque no tenga Internet.**

Base **C07/C11/C14**. Presentación/acceso **D01/D09**.
- Caddy, Pi-hole y el acceso HTTPS local no esperan mesh ni compute listo.
- El acceso del host verifica TLS y nombres propios; no requiere otra IP mesh.
- Antes de que la web pueda servirse, la CLI informa el avance; no se inventa otra GUI.

#### HU06 — Entender el avance y la acción pendiente
**Como operadora, quiero distinguir qué está disponible y qué falta para actuar
sin interpretar logs ni confundir un componente activo con un nodo sano.**

Base **C09/C14**. Presentación **D01**.
- Se separan acceso local, enrolamiento/conectividad mesh y compute.
- Se muestran fase, estado, causa/acción y antigüedad según el diseño acordado.
- Datos ausentes, antiguos o una observación fallida no se convierten en éxito.

#### HU07 — Enrolar Tailscale desde la CLI
**Como administradora, quiero introducir mi key mediante `gnx` para unir el
Tailscale de la WSL dedicada sin escribir la key en la web o el instalable.**

Base **C04/C14**. Retención de key **D06**, baseline **D07**.
- La entrada protegida llega al runtime de esa instalación, no a la imagen distribuida.
- El resultado distingue daemon iniciado de peer enrolado y acceso remoto verificado.
- Conservar el estado del peer no equivale a conservar la key de alta reusable.

#### HU08 — Seguir viendo la app si falla la mesh
**Como operadora, quiero mantener el estado local cuando falta red o la key es
inválida para corregir el problema sin reinstalar el nodo.**

Base **C14**.
- La app conserva avance, error o acción pendiente durante el fallo mesh.
- El fallo no detiene ni reinicia Caddy/Pi-hole ni autoriza purgar identidad o datos.
- No se afirma conectividad remota hasta comprobarla.

#### HU09 — Acceder a Proxmox
**Como operadora, quiero abrir `https://compute.gnx` para utilizar Proxmox con
su autenticación nativa sin publicarlo directamente en LAN o Internet.**

Base **C07/C11**. Modo compute/imagen **D02/D05**, acceso **D09**.
- El gateway valida TLS del backend; no usa un bypass de login, licencia o certificado.
- La contraseña se configura por la frontera protegida acordada, no mediante un default público.
- Compute pendiente no retira la app; VMs aceleradas no se prometen sin resolver D02.

### C. Operar y recuperar el nodo

#### HU10 — No mantener una terminal como supervisor
**Como operadora, quiero que servicio Windows, systemd y PVE sostengan sus
responsabilidades para no depender de una CLI permanente.**

Base **C05/C06/C11**.
- Se ejecutan los cuatro servicios base acordados: Tailscale, Pi-hole, Caddy y PVE.
- La recuperación corresponde a SCM/systemd/PVE, sin otro supervisor por aplicación.
- La recuperación conserva estado y no deshace una parada explícita del nodo.

#### HU11 — Iniciar el nodo con `gnx start`
**Como operadora autorizada, quiero iniciar el núcleo preparado con un comando
para recuperar su acceso sin volver a instalar o enrolar.**

Base **C17**; contrato en `architecture.md`, sección 6.
- Inicia el target propio mediante pipe e identidad dedicada, sin grupo `workloads`.
- Conserva datos, CA, credenciales e identidad; no aplica actualizaciones pendientes.
- Si falta preparación informa la causa; target activo no significa que todo esté sano.

#### HU12 — Detener el nodo con `gnx stop`
**Como operadora autorizada, quiero detener ordenadamente mi nodo para pausar
su actividad sin borrar datos ni afectar WSL ajenas.**

Base **C17**; contrato en `architecture.md`, sección 6.
- Detiene el target y servicios propios; PVE realiza la parada nativa de sus LXC.
- Se respeta stop tras la recuperación del servicio Windows; este conserva el canal de start.
- La web queda detenida por petición explícita; no hay `wsl --shutdown` global ni promesa de liberar toda la RAM.

#### HU13 — Consultar sin arrancar ni cambiar el nodo
**Como operadora, quiero consultar `gnx status` para conocer su situación sin
despertar una WSL detenida ni aplicar cambios.**

Base **C01/C17** y diseño de lectura existente; permisos **D08**.
- La consulta no instala, enrola, descarga imágenes ni reinicia servicios.
- Distingue parada, estado observado y antigüedad; no reutiliza salud anterior como actual.
- Un timeout se informa como resultado desconocido/fallo, no como éxito ni permiso de reset.

### D. Entregar servicios hacia LXC

#### HU14 — Entregar archivos desde la app
**Como operadora autorizada, quiero enviar Quadlets y configuración desde
`app.gnx` para desplegar servicios sin usar una segunda herramienta de despliegue.**

Base **C12**. Transporte/autorización **D12**, destino **D13**.
- La web entrega archivos; un path del navegador no concede acceso al disco del host.
- Usa la misma admisión y aplicación que la entrada CLI.
- Selector nativo, protocolo `gnx://` y ZIP no se dan por aprobados; no se heredan del legacy.

#### HU15 — Entregar un path desde la CLI
**Como operadora autorizada, quiero indicar un path local a Quadlets/configuración
para desplegar desde mis archivos sin recrearlos en un formulario.**

Base **C12**. Contrato de destino **D13**.
- Resuelve solo la entrada local autorizada y pasa por la misma admisión de HU14.
- No incorpora comandos `workload/workloads`, perfiles AI ni un catálogo heredado.
- La sintaxis concreta de entrega sigue sin fijarse; esta historia no inventa comandos o soporte ZIP.

#### HU16 — Construir solo cuando sea necesario
**Como operadora, quiero aportar `Containerfile` y contexto únicamente si mi
servicio necesita build para no reconstruir imágenes OCI ya disponibles.**

Base **C12**. Lugar/límites del build **D13**, aislamiento **P**.
- Una imagen disponible no obliga a build; el contexto solo se usa cuando corresponde.
- Quadlets y `Containerfile` se tratan como entradas capaces de ejecutar acciones,
  no como datos inocuos ni shell autorizado del nodo.
- El build no recibe sockets, pipes o secretos administrativos del núcleo;
  offline requiere también las bases/dependencias de ese contexto.

#### HU17 — Aplicar solo al destino autorizado
**Como administradora, quiero que la entrega se aplique al LXC acordado para
no modificar el núcleo ni un guest ajeno.**

Base destino **C12**; admisión/ownership **P**, creación/selección/recursos **D13**.
- Crear o seleccionar LXC y sus CPU/RAM/disco se resuelve antes de esa operación.
- Se validan contexto, rutas, privilegios, mounts y límites conforme al contrato acordado.
- PVE y los Quadlets del LXC gobiernan el servicio; no hay otro motor de lifecycle.

### E. Completar el primer caso real

#### HU18 — Desplegar tres aplicaciones en un LXC
**Como operadora, quiero desplegar RustFS, Pocket ID y NetBird juntos para
obtener el primer caso real mediante la misma entrada app/CLI.**

Base **C12/C13/C15**. Recursos/acceso **D13**, versiones/dashboard **D14**.
- Son aplicaciones posteriores al núcleo, no instancias que el instalador activa por defecto.
- Usan Quadlets del mismo LXC y el gateway/DNS existentes, sin sidecar mesh por guest.
- Tres aplicaciones no ocultan un cuarto contenedor: el dashboard oficial NetBird separado se decide en D14.

#### HU19 — Acceder a NetBird y RustFS mediante Pocket ID
**Como persona usuaria de esas aplicaciones, quiero autenticarme mediante
Pocket ID para utilizar su integración OIDC sin otro IdP adicional.**

Base **C15**. Versiones, clientes, roles y callbacks **D14**.
- Se verifican ambos flujos por separado, con clientes y permisos acordados.
- Pocket ID no autentica `app.gnx`, Proxmox, la CLI ni Tailscale.
- Login no sustituye verificar uso real: permisos RustFS y protocolos NetBird;
  passkey/consentimiento personal se registra pendiente si requiere al usuario.

### F. Conservar la instalación durante mantenimiento

#### HU20 — Distinguir actualizar de borrar o reinstalar
**Como administradora, quiero conocer qué paquete está instalado y qué runtime
está aplicado para autorizar mantenimiento sin perder identidad o datos.**

**P**: contrato de actualización/conservación descrito en arquitectura; no es
un requisito C nuevo. Convivencia, uninstall y purga pendientes en **D11**.
- Instalar un paquete no acredita que su runtime esté aplicado o sano.
- Una activación disruptiva requiere confirmación; status/start no son auto-updaters.
- Repair, fallo de health y desinstalación no autorizan limpieza implícita;
  la política de conservación y cualquier retirada se acuerdan antes de ejecutarlas.

## 5. Decisiones que impiden inventar detalles

No se repiten aquí sus propuestas completas: la autoridad sigue siendo la tabla
Dxx de `scope.md`. Esta es su relación con el recorrido, no otro registro de decisiones.

| Decisión | Afecta a |
| --- | --- |
| D01 | HU05–HU06: quién ve el estado y presentación del avance. |
| D02/D05 | HU09: modo compute, imagen exacta y credenciales. |
| D03 | Clientes tailnet externos y aceptación remota; no bloquea HU05. |
| D04 | Resuelta: Pi-hole; no volver a proponer otro DNS. |
| D06 | HU07: consumo/retención de key, distinto del estado persistente del peer. |
| D07/D10 | HU01: baseline, prerrequisitos, versiones y empaquetado offline. |
| D08 | HU04/HU13: permisos para lectura de estado. |
| D09 | HU05/HU09: publicación HTTPS Windows↔WSL. |
| D11 | HU01/HU03/HU20: convivencia, actualización, retirada y conservación. |
| D12 | HU14: transporte y autorización de la entrega web. |
| D13 | HU14–HU18: destino LXC, recursos, build y acceso. |
| D14 | HU18–HU19: dashboard NetBird, imágenes y configuración OIDC. |

ZIP, diseño de tarjetas/enlaces, eventos recientes, controles web de recursos,
cancelación y recorrido de actualización de aplicaciones no quedan aprobados
por este documento. No añadirlos como historias comprometidas por existir en
legacy o por haberlos mencionado en una comparación.

## 6. Límites transversales y trazabilidad

| Requisitos | Cobertura |
| --- | --- |
| C01/C02 | HU02 y restricciones de implementación: CLI mínima, Rust propio, archivos externos. |
| C03/C04 | HU03–HU04 y HU07: identidad, IPC y secretos. |
| C05/C06 | HU02/HU04/HU10–HU12/HU17: resiliencia y lifecycle nativos. |
| C07/C11/C14 | HU05–HU10: dos URLs, cuatro servicios, avance y acceso local independiente de mesh. |
| C08/C13 | HU01/HU18: MSI/EXE e inputs offline, sin instanciar apps por defecto. |
| C09 | Sección 7: verificación proporcional y documentación puntual. |
| C10 | Legacy como evidencia, nunca autoridad ni plantilla de migración; preservar su historia. |
| C12 | HU14–HU18: dos entradas, mismo despliegue LXC, build opcional. |
| C15 | HU18–HU19: caso real y OIDC solo entre las tres aplicaciones. |
| C16 | Hitos de hasta 20 minutos con resultado/evidencia; no SLA del producto completo. |
| C17 | HU11–HU13: start/stop del nodo y consulta sin efectos secundarios. |

Las exclusiones de `scope.md` se mantienen: no AI/Continuity/GitHub Actions,
OAuth/IdP para app, PWA offline, telemetría, auto-updater, catálogo, motor genérico
de pipelines o multiusuario/multinodo. Tampoco se autoriza restaurar legacy,
reescribir productos de terceros, añadir un backend propio en otro lenguaje o
crear mecanismos de agentes dentro del producto.

## 7. Cómo aceptar sin pruebas exhaustivas

Los criterios describen resultados; no obligan a crear un test automatizado por
frase ni un framework por historia. Se reutiliza evidencia pertinente y se
comprueban únicamente las fronteras afectadas por el cambio autorizado.

| Hito futuro | Evidencia mínima relevante |
| --- | --- |
| Núcleo instalado | Instalación real offline en baseline acordado; app HTTPS antes de key; key inválida/red ausente sin retirar la app; compute HTTPS y auth nativa. |
| Operación | stop/start reales, status sin despertar, parada respetada tras recuperación y datos/identidad conservados; sin interferir con WSL ajenas. |
| Fronteras modificadas | Pruebas pequeñas de autorización, secretos y admisión según el código afectado; no omitir denegaciones críticas. |
| Despliegue final | Entrega real desde app y desde CLI al mismo contrato; caso LXC, ambos logins OIDC, uso/salud de servicios y persistencia. |
| Mantenimiento, si se autoriza | Diferenciar instalado/aplicado y comprobar conservación según D11; sin probar mediante purga de la instalación existente. |

**Lista para implementar:** hito expresamente autorizado, objetivo C identificado
(o propuesta aprobada), decisiones dependientes resueltas, archivos/recursos y
ownership conocidos, cambio mínimo y comprobación definidos. Una pregunta abierta
detiene solo la parte dependiente, no trabajo independiente ya autorizado.

**Terminada:** resultado real comprobado y evidencia breve sin secretos;
limitaciones e interacción humana pendientes declaradas. Compilar, abrir un
selector o ver una pantalla de login no demuestra la entrega completa.

**Situación actual:** CLI/servicio Rust parcialmente implementados y compilados;
sin instalación nueva ni runtime preparado. Ninguna historia completa está
marcada probada o aceptada. Este documento no amplía la autorización.
