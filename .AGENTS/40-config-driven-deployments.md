# Paradigma obligatorio: despliegues config-driven y entrega verificada

Decisión de la persona operadora (2026-10-07). Leer después de `00-scope.md`,
antes de planear cualquier servicio, máquina, upgrade o requisito nuevo.
Este documento gobierna el trabajo de agentes; no afirma que el motor genérico
ni la carga de bundles desde el frontend estén implementados hoy.

## 1. Resultado esperado, no soluciones laterales

- Un nuevo servicio se expresa en configuración externa: perfil de máquina,
  Quadlets reales, imágenes por digest, paths, límites, persistencia, secretos
  referenciados, red, dependencias y verificaciones declarativas.
- Un nuevo LXC usa el mismo lifecycle de la plataforma. No añadir un controlador,
  un enum Rust, un comando especial, un branch por app ni otro provisioner por
  cada servicio. Cambiar el núcleo sólo cuando falte una capacidad genérica,
  justificada por un contrato y reutilizable por más de un perfil.
- Rust conserva seguridad Windows, IPC, validación y coordinación. Los Quadlets,
  reglas, configuración y conocimiento de aplicaciones no se incrustan en Rust.
- No hotfix al runtime: no copiar código editado al guest, no editar su DB/reglas
  para lograr verde y no ejecutar un runner alternativo para completar el alta.
  Corregir la fuente, construir, instalar y aplicar el payload por la CLI instalada.
- Un contenedor `Up`, un LXC `running`, una página de login o un dry-run Quadlet
  no constituyen la entrega. Exigir salud de las dependencias y uso real.

## 2. Un solo recorrido: fuente → artefacto → instalado → evidencia

1. Inspeccionar fuente, estado instalado, ownership, logs canónicos y espacio.
   Usar shell/CLI existente y browser skill cuando haga falta observar la UI.
2. Definir el cambio mínimo y los criterios de aceptación antes de editar.
   Consultar versiones y contratos upstream; no inventar flags/API/path/credenciales.
3. Editar sólo fuentes y contratos versionados del producto. Ejecutar pruebas
   estáticas, de comportamiento y negativas. Si falta una prueba recurrente,
   incorporarla a `tests/`, no escribir un side script descartable.
4. Verificar inputs offline, espacio máximo del build y cuota remota. Ejecutar
   `installer/scripts/build.ps1` y `installer/scripts/verify.ps1`.
5. Instalar el Burn resultante. Aplicar con los comandos de la CLI instalada y
   comprobar versión/hash deseado contra el realmente aplicado. No usar un EXE
   viejo para acreditar correcciones posteriores en fuente.
6. Validar setup inicial y upgrade por separado. Conservar guest/discos/identidad,
   CA y secretos existentes. Reiniciar por el lifecycle autorizado y volver a
   verificar. No presentar upgrade como prueba de instalación limpia.
7. Usar browser skill sobre Proxmox, accesos HTTPS y la aplicación cuando corresponda.
   Registrar lo realmente visible y los límites. No simular passkeys ni inferencia.
8. Guardar evidencia sanitizada durable en `docs/validation/<milestone>/` con
   commit, versión, hashes, comandos, resultados y bloqueadores. Nunca tokens,
   cookies, prompts secretos, dumps de env ni respuestas de auth completas.
9. Publicar la fuente y los artefactos mediante los destinos aprobados; descargar
   y verificar bytes/hash desde cada destino requerido. Limpiar los intermedios
   propios y reportar espacio final. No declarar terminado si falta un gate.

Si un gate falla, el resultado es `blocked/attention/failed`, no una alternativa
manual que sólo el agente sabe repetir. No publicar un candidato incompleto como
release validada. No reincidir en builds repetidos sin explicar el fallo anterior.

## 3. Sin archivos temporales de trabajo ni basura heredada

Prohibido crear runners, side scripts, clones scratch, reportes sueltos o árboles
paralelos en `/tmp`, Windows Temp, escritorio, raíz del repo o guest para resolver
la tarea. Usar comandos directos/stdio/memoria, herramientas existentes y tests
versionados. Un helper nuevo debe tener una responsabilidad permanente del
producto, ownership, tests y conexión con el lifecycle; no basta llamarlo `.run`.

Distinguir tres clases:

| Clase | Regla |
| --- | --- |
| Fuente/contrato | Versionado en paths del producto; no una segunda implementación |
| Evidencia final | Durable, sanitizada, en `docs/validation/`; no runner ejecutable |
| Intermedio de herramienta | Sólo workspace administrado de build/test, inventariado y limpiado |

Los intermedios que generan compiladores, WiX, Podman o la herramienta browser
no autorizan al agente a escribir scripts ad hoc. Los archivos atómicos y bundles
one-shot de secretos que exige el contrato instalado son mecanismo del producto,
no archivos de trabajo del agente: ACL/permisos antes de escribir, no exposición,
limpieza en éxito/error/reinicio. No romper ese contrato por una prohibición literal
que empuje credenciales a argv/env. Nunca copiar secretos a evidencia ni Git.

- Cada build debe registrar entradas/salidas, pico previsto, cachés y responsables.
- Reutilizar inputs sellados direccionados por hash; no exportar/importar o copiar
  imágenes idénticas por app/scope. No conservar MSI público además de Burn.
- Retener sólo último artefacto validado, candidato vigente y evidencia necesaria
  conforme a la política de releases. Limpiar outputs fallidos propios tras
  guardar diagnóstico. No borrar archivos ajenos/untracked por ser untracked.
- No usar `git clean`, prune global, borrar journals/volúmenes ni eliminar WSL
  productiva para resolver falta de espacio. Identificar dueño y alcance primero.
- Antes de retirar una distro de builds: inventario durable, fuentes no publicadas,
  credenciales a preservar, artifacts pendientes, disco y cachés; comprobar el
  entorno sustituto y retirar sólo el nombre/SID/ruta autorizados. No `wsl --shutdown`.

## 4. Tailscale: una clave reusable, identidades separadas

La persona operadora establece que la clave de setup es **reusable** y sirve
para enrolar sidecars nuevos durante alta/despliegue/upgrade. El objetivo es no
volver a pedir una clave por LXC ni añadir un flujo manual `join` por aplicación.

- Guardar la clave reusable sólo en el secret store protegido de la plataforma;
  nunca en el perfil público, browser, argv, URL, logs, Git o variables de entorno.
- Pasarla por archivo protegido al sidecar, retirando únicamente el staging.
  Conservar la fuente protegida mientras esté vigente y autorizada.
- Cada LXC tiene hostname/estado/identidad Tailscale propios. Reutilizar la clave
  no significa copiar `tailscaled.state` ni compartir identidad entre máquinas.
- Si un sidecar ya está inscrito, un upgrade/restart conserva esa identidad y no
  ejecuta `tailscale up` de nuevo. Un peer nuevo sí utiliza la clave reusable.
- Comprobar capacidad/validez de la clave mediante contrato real y resultado;
  el prefijo `tskey-auth-` no prueba que sea reusable ni que no esté expirada.
- Si la instalación actual consumió y borró la clave, no fingir que aún existe.
  Diseñar una migración de secret store con captura interactiva una sola vez y
  consentimiento. Caducidad/revocación es un bloqueo explícito, no rekey silencioso.

El comportamiento vigente consume la clave; este cambio requiere implementación,
pruebas y una nueva instalación validada. No documentarlo como ya funcionando.

## 5. Declaración, reconciliación y observabilidad genéricas

El motor objetivo admite un registro de perfiles, no la allowlist fija `rustfs|ai`.
El mismo recorrido valida, planifica, crea/asigna ownership, prepara offline,
enrola un mesh nuevo, aplica Quadlets, verifica y registra la revisión aplicada.

- Paths confinados a la máquina y servicio declarados. No symlinks, `../`, devices,
  mount del host, scripts de arranque arbitrarios ni secrets inline de un upload.
- Unprivileged LXC, límites CPU/RAM/disco, redes/UID/capacidades y volúmenes
  explícitos. Fallo cerrado para opciones no soportadas.
- Recibo persistente: ID lógico, VMID, owner, revisión deseada/aplicada, hashes,
  estado observado, recursos y último error. Nunca adoptar guests sin propiedad.
- En cada restart/boot, verificar ownership, revisión e integridad, unidades,
  sidecar, DNS/TLS, auth y salud declarada. No recrear discos ni reconciliar una
  máquina cuyo estado deseado sea `stopped`. Lecturas no despiertan WSL/guests.
- CLI y frontend muestran la misma operación/proyección y logs sanitizados con
  límites/cursor. No traducir un error de observación a `available/healthy`.
- La UI no es un shell root. La carga de estructuras Quadlet desde el frontend
  es el objetivo aprobado, pero exige un contrato de admisión y autorización
  explícita antes de ejecutar. Conservar Named Pipe y onboarding privilegiado;
  no crear a escondidas un control plane HTTP localhost.

Diseño y gates: [propuesta de motor genérico](../docs/config-driven-platform.md).

## 6. Publicación: distinguir GitHub, Hugging Face y S3

- El remote fuente actual es el Space HF `gnx-labs-mx/quetzalcoatl`, no GitHub.
  GHCR/GitHub del componente compute no equivale a publicación del instalador.
- Conservar publicación HF existente (Space/dataset según cuota) y sus revisiones
  inmutables. Usar identidad host de publicación, nunca `gnxnodesvc`.
- La persona operadora requiere además artefactos/releases e inputs offline en
  el S3 existente. Fijar endpoint, bucket/prefijo, CA y credencial de publicación
  aprobados antes de subir; no inventarlos ni usar la credencial AI de consumo.
- Manifiesto inmutable: commit/snapshot fuente, versión, hashes, tamaños, imágenes
  OCI, inputs offline, requisitos, evidencia y enlaces. S3 de blobs no es registry OCI.
- Confirmar upload, descarga/readback por hash y reproducción del build/instalación.
  Un helper que genera notas no prueba instalación, salud ni publicación S3.
- Si se requiere también un remote GitHub, registrar su destino aprobado; no
  trasladar la fuente ni crear un repo/cuenta automáticamente.

## 7. Coordinación y cierre

Agentes de implementación: `gpt-5.6-luna`, esfuerzo **high**. No sustituir por otro
modelo/esfuerzo silenciosamente; si no está disponible, informar el bloqueo.
Un coordinador controla build/install/aceptación y cleanup; workers tienen paths
no solapados y no mutan el runtime. No prometer que una sesión existente cambió
su modelo sólo porque se editó este documento.

Cierre mínimo: qué fuente cambió, versión/hash construida e instalada, pruebas y
browser evidence, comportamiento tras restart, destinos realmente verificados,
intermedios retirados, espacio final y gate faltante. No confundir propuesta,
contratos estáticos, preparación offline y aceptación completa.
