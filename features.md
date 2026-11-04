# Features de Quetzalcoatl

Requisitos pendientes y prioridades de diseño, no funciones implementadas ni
aceptación. [scope.md](scope.md) delimita el producto actual; las decisiones aquí
no autorizan instalar, publicar, migrar datos ni ampliar permisos por sí solas.

## Prioridad de entrega

La entrega inmediata es una demo funcional de `app.gnx`, con presupuesto máximo
de **un día** y un primer recorrido utilizable en horas, no una planificación
semanal. P0 define el corte de esa entrega; P1/P2 no lo bloquean. Los requisitos
de actualización se conservan, pero no son prerrequisitos de la demo. Integridad,
autorización, protección de secretos y conservación de datos no se posponen.

| Orden | Feature | Resultado esperado |
| --- | --- | --- |
| P0 | R-OCI-01 — Construcción y ejecución Podman | Containerfiles versionados, imágenes identificables y servicios bajo Quadlets en el LXC existente autorizado. |
| P0 | R-INF-01 — Inferencia mediante OmniRoute | Endpoint privado compatible OpenAI, proveedores autorizados, cuotas y política explícita de coste y privacidad. |
| P0 | R-WEB-01 — Interfaz conversacional GnX | Adoptar la experiencia de `openui-f1-dashboard`: dashboard, conversación, herramientas visibles y respuestas OpenUI en streaming. |
| P0 | R-AGENT-01 — Consulta con datos reales | Una herramienta de solo lectura, tabla/gráfico y seguimiento de conversación, sin control administrativo. |
| P1 | R-OBS-01 — Kluster y Kubeshark | Ampliar consultas de infraestructura y red sin exigir Kubernetes ni privilegios eBPF al recorrido P0. |
| P1 | R-UPD-01 — Actualizaciones como estado deseado | Saber qué hay instalado, aplicado y ejecutándose; adoptar una entrega compatible sin reinicios manuales a ciegas. |
| P1 | R-IMG-01 — Actualización de imágenes | Cambiar imágenes del núcleo y stacks por digests aprobados, sin recrear guests ni confundirlo con actualizar GnX. |
| P2 | R-MOBILE-01 — Clientes móviles | Adoptar Appless como referencia de desarrollo móvil, bajo el mismo estándar OpenUI. |

La recuperación de base LXC (**R-LXC-01**, en alcance) sigue pendiente y condiciona
las actualizaciones que necesiten cambiar esa base; no equivale a rollback de datos.

### Corte mínimo y orden de trabajo

La referencia principal es [OpenUI F1 Dashboard](https://github.com/thesysdev/openui-f1-dashboard),
la demo completa derivada del [cookbook de analítica conversacional](https://github.com/thesysdev/openui/tree/main/examples/cookbooks/conversational-analytics).
No se diseña otra portada primero ni se copian todas las páginas de Fórmula 1.
Se conserva la interacción y se sustituye su dominio por información GnX.

```text
Fuentes y Containerfiles versionados
             │ construcción con Podman en builder autorizado
             ▼
Imágenes OCI identificadas → systemd/Quadlets → Podman dentro del LXC existente
                                                │
https://app.gnx → Caddy existente → aplicación GnX con OpenUI
                                      ├─ backend → OmniRoute /v1 → proveedores
                                      └─ consulta acotada → datos reales autorizados
```

El repositorio de integración es [gnx-kluster](https://github.com/mayas-alas/gnx-kluster)
(nombre del repositorio; «cluster» describe aquí el conjunto de servicios con
engine **Podman**). No se introduce Docker Engine, Compose, Kubernetes, Helm ni
un supervisor adicional como requisito de ejecución. El origen kftray de ese fork
no acredita soporte web o gestión de Podman ya implementados. Estas decisiones
son requisitos pendientes; este checkout no contiene todavía esa integración.

| Ventana máxima de trabajo | Resultado que debe obtenerse |
| --- | --- |
| 0–1 h | Identificar checkout/revisión y licencia de las referencias, builder Podman, LXC propio, recursos, acceso al gateway y proveedor/modelo autorizados. Resolver el canal protegido de credenciales sin pedir secretos por chat. |
| 1–4 h | Obtener imágenes trazables desde Containerfile y OmniRoute accesible privadamente bajo Quadlet, con un proveedor operativo. Primer artefacto ejecutable, no solo un diseño. |
| 4–8 h | Conectar la interfaz elegida a OmniRoute y una consulta real; mostrar respuesta OpenUI y actividad de herramientas. |
| 8–12 h | Cerrar HTTPS en `app.gnx`, historial propio, acceso protegido, límites y un seguimiento de conversación con tabla/gráfico verificable. |
| 12–24 h | Margen para corregir bloqueos del recorrido P0 y realizar su aceptación manual autorizada; no abrir P1/P2 antes del cierre. |

Son presupuestos, no evidencia ni garantía de finalización. Una credencial no
disponible, incompatibilidad o licencia sin aclarar se comunica en la primera
ventana afectada; no se consume el día en reintentos ni se sustituye por mocks.
La primera conexión usa un proveedor explícito; una API operativa no acredita
que las suscripciones y free tiers restantes estén conectados o permitidos.

El corte exige una portada/dashboard mínimo con entrada conversacional, una
tabla y un gráfico basados en una consulta real, actividad de herramientas,
seguimiento e historial propio. Los datos incluyen fuente, momento de lectura y
ausencias/errores explícitos. No se generan métricas plausibles para completar
una pantalla. Páginas adicionales, catálogo de apps, móvil, orquestación genérica,
control administrativo y captura eBPF no forman parte del corte.

Antes de aplicar al host se identifican VMID/propiedad, unidades y datos afectados,
publicación `app.gnx`, interrupción prevista y vía de restitución. No se recrea
el LXC ni se ejecuta `gnx deploy` con una carpeta cambiada para actualizarlo. El
gateway actual sirve una página estática: su publicación dinámica necesita una
adaptación explícita y autorizada, conservando CA, CRL y acceso informativo cuando
el backend no esté disponible. No se reconstruye MSI/EXE para documentar la demo.

El cierre funcional está en [los recorridos pendientes](historias-de-usuario.md#demo-conversacional-appgnx).
Construido, desplegado, accesible y aceptado se informan por separado; una página
visible o una unidad activa no acreditan el recorrido completo.

## R-OCI-01 — Containerfile, Podman y Quadlets

- Versionar Containerfiles de aplicación/backend y la construcción de OmniRoute
  cuando se use el fork seleccionado. Fijar revisión, bases y dependencias con
  lockfile; revisar licencias antes de incorporar código/assets. Una imagen de
  upstream no acredita contener las modificaciones de otro fork.
- Construir con Podman en un builder autorizado, separado del servicio web y sin
  montar su socket en la aplicación o el gateway de inferencia. Si el builder está
  en el LXC, comprobar recursos y compatibilidad sin elevarlo a privilegiado.
  Construir allí no es requisito para ejecutar allí las imágenes.
- Entregar imagen por registro confiable con digest verificable. Una transferencia
  OCI nativa requiere un recorrido de mantenimiento autorizado distinto del bundle
  de `gnx deploy`; la CLI actual no admite archivos OCI ni Containerfiles.
- Ejecutar con engine Podman y unidades `.container`/`.network` de Quadlet;
  systemd posee arranque, parada y recuperación. Sin builds en cada petición o
  arranque, `.build` supuesto, Compose, PM2 ni motor nuevo de workloads.
- Persistir conversaciones y estado privado de OmniRoute fuera de las capas de
  imagen, con ACL/permisos y copia recuperable antes de cambios con estado. Claves
  fuera de contexto de build, capas, argumentos y repositorio; cifrado de estado
  de OmniRoute configurado con clave protegida separada, no asumido por defecto.
- Ajustar escucha dentro del contenedor para el proxy/red privada; los scripts
  loopback de la referencia no se publican simplemente con un mapeo de puerto.
  Gateway y backend de inferencia no se exponen como administración pública.

## R-INF-01 — OmniRoute como gateway de inferencia

**OmniRoute es P0**, no una integración opcional posterior. Usar el
[fork seleccionado](https://github.com/ChrisCompton/omniroute), fijando revisión y
procedencia de imagen. Su papel en la demo es inferencia; no se incorporan sus
capacidades de ejecución, túneles públicos, MCP/A2A o gestión de procesos por rutina.

- Contrato inicial: `GET /v1/models` y `POST /v1/chat/completions`, streaming y
  function tools en el modelo elegido. Responses es una extensión posterior salvo
  necesidad concreta; tener una ruta no demuestra todas sus semánticas por proveedor.
- La aplicación conserva su bucle acotado de herramientas y transforma respuestas
  del gateway en eventos AG-UI para el navegador. OpenAI-compatible, AG-UI y OpenUI
  Lang son contratos distintos: API de inferencia, eventos y descripción visual.
- Configurar endpoint/modelo en servidor; ninguna clave de proveedor o de OmniRoute
  en el frontend. Limitar entrada, concurrencia, rondas, tokens, tiempos y gasto.
  Acceso del operador protegido y ownership de cada conversación en el servidor;
  `Origin`, CORS o un ID anónimo de sessionStorage no autentican a una persona.
- Admitir suscripciones y APIs con free tier proveedor por proveedor según permiso,
  disponibilidad, capacidades y tratamiento de datos. La compatibilidad técnica
  de un adaptador no autoriza reutilizar credenciales, compartir cuentas ni revender
  cuota. No prometer uso ilimitado ni cuotas del README como compromiso de GnX.
- Para OpenAI, comprobar elegibilidad de la vía oficial de uso de planes en apps
  participantes o créditos API asociados a suscripción; activación real y facturación
  API son distintas de tener un plan. El adaptador Codex no acredita esa elegibilidad.
  No usar credenciales Claude de suscripción como proxy para terceros sin autorización
  expresa del proveedor; conservar su vía API admitida.
- Política de rutas explícita: al agotar cuota, informar; fallback solo a destinos
  previamente permitidos por capacidad, privacidad y presupuesto. Nada de pasar a
  pago o a un free tier que cambie tratamiento de datos silenciosamente. Respetar
  `Retry-After`; no repetir herramientas ni reiniciar una respuesta parcial a ciegas.
- Empezar con un proveedor autorizado. Otro proveedor puede añadirse después de
  ese recorrido; no se declara cobertura general de suscripciones/free tiers por
  mostrar un selector. Sin proveedor autorizado disponible, la inferencia queda
  bloqueada y no se finge una respuesta ni se adopta Thesys Cloud como atajo.

## R-UPD-01 — Nuevo enfoque de actualizaciones

**Primer incremento en fuentes 0.0.20:** comparación local al arrancar y operaciones
manuales `update --check|--apply [catalogo.json]` para las ocho unidades conocidas.
El administrador aporta digests revisados; no se han implementado descubrimiento
remoto, un feed firmado ni aplicación automática. [Contrato y límites](docs/updates.md).
La consola RustFS v0.1.38 queda incorporada al payload; construir estas fuentes no
acredita todavía instalación ni aceptación del actualizador.

**Propuesta prioritaria:** entregas declarativas y versionadas, con un catálogo o
manifiesto de estado deseado. No usar `latest`, búsquedas improvisadas de tags en
cada inicio ni el número de versión del CLI como prueba de que todo se actualizó.
El esquema y el mecanismo de confianza deben definirse antes de implementarlo.

- CI prepara la entrega a partir de versiones oficiales y fija versiones y digests
  por componente. Registrar la versión de consola embebida cuando determine si una
  corrección está incluida, como ocurre con RustFS.
- Separar cuatro capas: producto GnX/MSI, imágenes del núcleo, aplicaciones en LXC
  y filesystem/base. Cada una muestra su versión y estado, sin un único «actualizado»
  que oculte componentes pendientes o fallidos.
- Comprobar novedades en cada arranque real sin hacer depender el acceso local de
  Internet. La política externa define aplicación automática y ventana de mantenimiento
  o aprobación explícita; una descarga fallida conserva la versión conocida y lo informa.
- Permitir consultar novedades y solicitar actualización sin obligar al usuario a
  hacer `stop`/`start` como mecanismo de mantenimiento. Rust coordina una operación
  acotada; MSI, systemd/Quadlets y PVE siguen siendo dueños del lifecycle.
- Antes de aplicar, mostrar versiones anterior/propuesta, cambios, compatibilidad,
  interrupción prevista y límites de recuperación. Descargar y verificar antes
  del cambio; no detener servicios para descubrir después que falta un artefacto.
- Definir autenticidad de la entrega, no solo hashes. Conservar semilla offline,
  referencias anteriores y datos; no prometer revertir migraciones con un rollback
  de imagen. Exigir recuperación adecuada cuando una migración no sea reversible.
- Cerrar con evidencia de la configuración aplicada y de la imagen que ejecuta
  cada servicio; distinguir salud funcional de integridad y de unidad activa.
  Un timeout conserva un resultado desconocido y no dispara otra mutación a ciegas.
- Actualizar el guest propio identificado por recibos/VMID; reenviar una carpeta
  distinta a `gnx deploy` no es actualizar el guest existente. Respetar paradas
  explícitas y no reenrolar identidades ni rotar secretos.

**Criterio causal para RustFS:** no anunciar corregido el refresco continuo solo
porque se instaló otro GnX o se descargó una imagen. Comprobar que la imagen incluye
la corrección de consola y, por separado, el comportamiento real con autorización.
Si todavía no existe una imagen compatible corregida, comunicar ese bloqueo.

## R-WEB-01 — `app.gnx`: dashboard conversacional OpenUI

- **OpenUI es el estándar del frontend de Quetzalcoatl**, también para futuros
  frontends custom. Appless no sustituye ese estándar ni es la base web obligatoria.
- Referencia literal de experiencia: **OpenUI F1 Dashboard** como dashboard
  conversacional. Conservar dashboard, navegación lateral, bienvenida,
  entrada persistente, conversaciones y respuestas visuales; sustituir identidad,
  assets y herramientas F1 por GnX, sin copiar marcas o páginas sin utilidad.
- Para la demo basta un dashboard y conversación con seguimiento. Accesos a las
  interfaces nativas son secundarios y conservan sus autenticaciones, con hover,
  foco y alternativa táctil accesibles; no se construye un catálogo antes del chat.
- Config-first: definir tokens de diseño, componentes admitidos, propiedades,
  widgets, destinos y acciones como contrato versionado. Generar desde la misma
  biblioteca la especificación/prompt del agente y el renderer; evitar contratos
  duplicados o HTML/JavaScript arbitrario generado por el modelo.
- Usar OpenUI Lang para la descripción de UI generada; no eliminar JSON de toda
  configuración solo por comparar formatos. No atribuir a GnX las cifras de
  rendimiento que publica el proveedor sin medición propia.
- Mostrar estado real y errores/carga/desconexión explícitos, no datos plausibles
  generados como si fueran observaciones del nodo. No recoger secretos administrativos.
- Los componentes de datos reciben referencias acotadas y consultan endpoints de
  lectura autorizados; no aceptan URLs, SQL o comandos arbitrarios del modelo.
  Preferir que las series lleguen de la fuente al gráfico sin reescritura del LLM.
  Validar también las cifras de tablas copiadas desde resultados de herramientas.
- Mantener identidad/autoría y licencia del código reutilizado sin inferir que el
  repositorio de la demo hereda automáticamente la licencia del proyecto principal.

## R-AGENT-01 — Agentes que responden con widgets

Usar el enfoque oficial **Build agents** de OpenUI: el backend incorpora el contrato
de componentes y el cliente renderiza sus respuestas en streaming y devuelve las
interacciones. **OmniRoute** es la vía de inferencia elegida; el proveedor/modelo
concreto requiere disponibilidad y permiso. La adaptación de aplicación se integra
en `gnx-kluster`; no se reescribe todo el frontend Tauri ni el núcleo Windows para
mostrar el primer recorrido. Rust conserva las fronteras y coordinación del producto;
el bucle de conversación de la aplicación no se convierte en supervisor del host.

- Conservar el flujo **AG-UI sobre SSE** de `openui-f1-dashboard`, incluida actividad
  y resultados de herramientas. El cookbook pequeño usa el stream NDJSON del runner:
  no intercambiar sus adaptadores ni conectarlos al SSE OpenAI sin conversión.
- Sustituir llamadas a OpenUI Gateway, historial remoto, frontend tokens y el modo
  de prompt `cloud: true` por contratos compatibles con la ejecución propia. No
  basta cambiar `baseURL`. Generar el prompt desde la biblioteca local y no asumir
  reparación automática o persistencia provistas por OmniRoute.
- Historial propio en servidor, con almacenamiento persistente y validación de
  acceso por conversación; navegador sin conexión directa a Thesys. No habilitar
  PostHog, sinks Cloud u otra telemetría por copiar dependencias de la referencia.
- Herramienta inicial: consulta agregada de inferencia desde la API nativa de
  OmniRoute, previa confirmación de sus campos y acceso. Si no ofrece la serie
  necesaria, elegir una fuente real ya disponible y autorizada antes de implementar;
  no inventar métricas ni crear un recolector adicional para salvar la pantalla.

- Presentar agentes y sus respuestas mediante tarjetas/widgets del sistema GnX.
  Hover/foco puede dar información o previsualización; **no ejecuta acciones
  administrativas ni crea agentes privilegiados** por el mero hecho de pasar el cursor.
- Separar renderizar una respuesta de crear/ejecutar un agente. OpenUI no aporta por
  sí solo provisioning, autorización, herramientas ni datos reales del núcleo.
- Definir herramientas permitidas, autenticación y confirmación de acciones antes
  de habilitar control. La página actual es informativa y no existe todavía ese
  canal administrativo web; una UI generada no amplía permisos del usuario.
- No exponer keys en bundles/configuración pública ni asumir dependencia obligatoria
  de un gateway externo. La adopción de servicios, coste y tratamiento de datos
  requiere una decisión explícita.

## R-OBS-01 — Kluster y Kubeshark sin cambiar de engine

- `gnx-kluster` es el destino de integración de la experiencia y servicios con
  Podman. Su origen kftray aporta código de acceso Kubernetes/Tauri y MCP, no una
  API de gestión Podman ya disponible. Separar adaptadores web/escritorio y no
  trasladar kubeconfigs, sockets o IPC privilegiado al navegador.
- [gnx-kubeshark](https://github.com/mayas-alas/gnx-kubeshark) aporta observación de
  red para Kubernetes y consultas MCP. No demuestra captura de Podman en un LXC.
  Verificar entorno, kernel, eBPF y alcance antes de decidir su incorporación.
- No instalar Kubernetes, conceder capacidades adicionales ni convertir el guest
  en privilegiado para completar esta demo. Sin compatibilidad comprobada, se
  informa integración pendiente; no se muestran datos simulados ni se bloquea P0.
- La captura y su retención requieren autorización explícita: no enviar PCAP,
  cabeceras, tokens o payloads a proveedores externos por defecto. Desactivar
  telemetría no autorizada antes de aplicar una configuración de captura.

## R-MOBILE-01 — Appless como referencia móvil

Integrar/adaptar el paradigma de [Appless](https://github.com/thesysdev/appless) para
clientes móviles de Quetzalcoatl, compartiendo contrato de diseño con OpenUI. No
limitar por ello la mejora web a Android. Localizar el cliente móvil existente antes
de elegir su migración; su compatibilidad no se presume por el nombre del proyecto.

Appless upstream usa React Native/Expo y Material 3 en Android, pero se presenta
como experimento con acciones simuladas y generación mediante un modelo externo.
No trasladar esas simulaciones a estado real de GnX. Definir servicio de inferencia,
autenticación, licencias/dependencias y comportamiento sin conexión antes de integrar;
no habilitar telemetría remota ni empaquetar claves como variables públicas.

## Fuentes de diseño

- [OpenUI, proyecto estándar elegido](https://github.com/thesysdev/openui).
- [OpenUI F1 Dashboard, referencia principal de experiencia](https://github.com/thesysdev/openui-f1-dashboard).
- [Cookbook de analítica conversacional, referencia mínima de flujo](https://github.com/thesysdev/openui/tree/main/examples/cookbooks/conversational-analytics).
- [OmniRoute, fork elegido para inferencia](https://github.com/ChrisCompton/omniroute).
- [GnX Kluster, repositorio de integración](https://github.com/mayas-alas/gnx-kluster).
- [GnX Kubeshark, observación de red](https://github.com/mayas-alas/gnx-kubeshark).
- [OpenAI: uso de planes en aplicaciones participantes](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites).
- [OpenAI: créditos API para suscripciones elegibles](https://help.openai.com/en/articles/20001626-api-credits-benefit-for-eligible-chatgpt-subscriptions).
- [Anthropic: autenticación y uso de credenciales](https://code.claude.com/docs/en/legal-and-compliance).
- [Gemini API: condiciones de tratamiento de datos](https://ai.google.dev/gemini-api/terms).
- [OpenUI Lang y su relación con JSON](https://www.openui.com/docs/openui-lang).
- [Contrato de componentes y especificación para el agente](https://www.openui.com/docs/build-agents/ui-generation).
- [Build agents: alcance de backend y frontend](https://www.openui.com/docs/build-agents).
- [Comparación propuesta OpenUI/JSON](https://www.openui.com/openui-vs-json).
- [Appless: alcance móvil y límites del experimento](https://github.com/thesysdev/appless).
