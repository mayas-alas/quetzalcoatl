# Features de Quetzalcoatl

Requisitos pendientes y prioridades de diseño, no funciones implementadas ni
aceptación. [scope.md](scope.md) delimita el producto actual; las decisiones aquí
no autorizan instalar, publicar, migrar datos ni ampliar permisos por sí solas.

## Prioridad

| Orden | Feature | Resultado esperado |
| --- | --- | --- |
| P0 | R-UPD-01 — Actualizaciones como estado deseado | Saber qué hay instalado, aplicado y ejecutándose; adoptar una entrega compatible sin exigir reinicios manuales a ciegas. |
| P0 | R-IMG-01 — Actualización de imágenes | Cambiar imágenes del núcleo y stacks existentes por versiones/digests aprobados, sin recrear guests ni confundirlo con actualizar GnX. |
| P1 | R-WEB-01 — Inicio GnX con OpenUI | `https://app.gnx` como entrada de texto, widgets y accesos a aplicaciones. |
| P1 | R-AGENT-01 — Respuestas de agentes con interfaz | Renderizar respuestas interactivas con componentes propios y acciones autorizadas. |
| P2 | R-MOBILE-01 — Clientes móviles | Adoptar Appless como referencia de desarrollo móvil, bajo el mismo estándar OpenUI. |

La recuperación de base LXC (**R-LXC-01**, en alcance) sigue pendiente y condiciona
las actualizaciones que necesiten cambiar esa base; no equivale a rollback de datos.

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

## R-WEB-01 — `app.gnx`: texto, widgets y aplicaciones

- **OpenUI es el estándar del frontend de Quetzalcoatl**, también para futuros
  frontends custom. Appless no sustituye ese estándar ni es la base web obligatoria.
- Entrada de texto central para interactuar con GnX, no un buscador Google.
  Estilo de página de inicio inspirado en la referencia visual de Firefox:
  identidad GnX, superficies sobrias, accesos rápidos y widgets; sin copiar sus
  marcas ni convertirlo en un navegador nuevo.
- Tarjetas de aplicaciones con icono, nombre y descripción/preview en hover y foco;
  alternativa accesible para teclado y pantalla táctil. Enlaces a las interfaces
  nativas, por ejemplo `https://rustfs.gnx`, sin reemplazar sus autenticaciones.
- Config-first: definir tokens de diseño, componentes admitidos, propiedades,
  widgets, destinos y acciones como contrato versionado. Generar desde la misma
  biblioteca la especificación/prompt del agente y el renderer; evitar contratos
  duplicados o HTML/JavaScript arbitrario generado por el modelo.
- Usar OpenUI Lang para la descripción de UI generada; no eliminar JSON de toda
  configuración solo por comparar formatos. No atribuir a GnX las cifras de
  rendimiento que publica el proveedor sin medición propia.
- Mostrar estado real y errores/carga/desconexión explícitos, no datos plausibles
  generados como si fueran observaciones del nodo. No recoger secretos administrativos.

## R-AGENT-01 — Agentes que responden con widgets

Usar el enfoque oficial **Build agents** de OpenUI: el backend incorpora el contrato
de componentes y el cliente renderiza sus respuestas en streaming y devuelve las
interacciones. La elección de proveedor/modelo y backend de agentes queda pendiente.

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
- [OpenUI Lang y su relación con JSON](https://www.openui.com/docs/openui-lang).
- [Contrato de componentes y especificación para el agente](https://www.openui.com/docs/build-agents/ui-generation).
- [Build agents: alcance de backend y frontend](https://www.openui.com/docs/build-agents).
- [Comparación propuesta OpenUI/JSON](https://www.openui.com/openui-vs-json).
- [Appless: alcance móvil y límites del experimento](https://github.com/thesysdev/appless).
