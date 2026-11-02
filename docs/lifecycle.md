# Propuesta de síntesis de lifecycle

**Estado: propuesta de diseño, no implementación.** Su aprobación documental no
autoriza cambios de runtime, limpieza, build, instalación, publicación ni nuevas
pruebas. El alcance vigente está en [scope](../scope.md), la implementación actual
en [architecture](../architecture.md) y la retirada en [uninstall](uninstall.md).

## 1. Objetivo y límites

Reducir código ejecutable y caminos de decisión de actualización, desinstalación
y recreación, reutilizando las fronteras existentes. No trasladar complejidad a
scripts, configuración, otro worker o un framework; tampoco juntar archivos solo
para bajar su cantidad. No restaurar fuentes históricas ni retirar funciones sin
una decisión explícita de alcance.

MSI/Burn conserva la transacción Windows; SCM el servicio; systemd/Quadlets y PVE
el lifecycle Linux. Un solo bootstrap prepara el nodo. La observación de propiedad
es común; las políticas de conservación y borrado permanecen diferentes.

## 2. Línea base del repositorio

Referencia reproducible: `b794df80f12def9b39bd9a8828bf38a177138c32` (fuentes 0.0.27).
Inventario de archivos seguidos por Git, antes de ampliar este documento:

| Categoría | Archivos | Líneas físicas |
| --- | ---: | ---: |
| Rust del producto | 23 | 8.051 |
| Operaciones Linux, `scripts/*.run` | 9 | 709 |
| Build/operaciones PowerShell | 3 | 333 |
| Instalador WiX/XML | 4 | 78 |
| UI HTML estática | 1 | 21 |
| Workflow CI | 1 | 283 |
| **Código y automatización** | **41** | **9.475** |
| Configuración de producto y stacks | 27 | 471 |
| Manifiestos Cargo y lockfile | 3 | 432 |
| Documentación vigente | 12 | 1.856 |
| Changelogs históricos | 22 | 579 |
| Contrato/configuración/skills de agentes | 8 | 343 |
| Metadatos Git | 2 | 29 |
| Licencia | 1 | 661 |
| Intermedios generados versionados | 10 | 234 |
| CLI históricos versionados | 21 | — |
| **Total seguido por Git** | **147** | **14.080** |

Las líneas incluyen comentarios, blancos y comprobaciones existentes; no son una
medida de complejidad ni aceptación. Runtime = Rust + operaciones Linux + HTML:
**8.781 líneas**. Los 10 intermedios incluyen dos binarios sin LOC; los 21 CLI
ocupan 18.088.960 bytes. Se excluyen `.git`, `target`, `build` ignorado, VHD y
evidencia externa. Un changelog histórico no UTF-8 se contó por saltos de línea sin
reescribirlo. La ampliación de esta propuesta cambia solo la cifra documental.

El mayor salto Rust fue 0.0.22 → 0.0.23: +2.176 líneas netas. Después de la primera
interrupción documentada en 0.0.24 se añadieron +806 netas hasta 0.0.27. Esto orienta
la revisión; no demuestra que una versión completa fallase siempre ni permite
reconstruir un error histórico cuya salida no se conservó.

## 3. Contratos que se conservan

| Operación | Recorrido y política |
| --- | --- |
| Actualizar | Verificar → instalar por MSI/Burn cuando corresponda → aplicar el paquete; conservar identidad, configuración, volúmenes y guests. Mantener el preflight de guests y la intención de parada. |
| Desinstalar | Verificar → detener lo propio existente → retirar publicación → MSI/Burn retira Windows. Conservar cuenta, perfil, WSL/VHD, CA y datos; no exigir bootstrap Linux completo. |
| Reset (`update --force`) | Inventariar y confirmar → revalidar SID/GUID/BasePath → `wsl --unregister` propio → comprobar ausencia → mismo bootstrap. La autorización incluye los datos y guests del VHD administrado; nunca recursos externos. |
| Preparación pendiente | Solo con ausencia nativa verificada y consentimiento `PREPARAR`: mismo bootstrap, sin repetir `unregister`. Un registro residual o nodo parcial no equivale a ausencia. |

Un VHD posterior al reset es nuevo, no una copia conservada. Un resultado nativo
desconocido bloquea repetición; cancelar no convierte una operación pendiente en
completada. Propiedad observable y autorización siguen siendo requisitos incluso
cuando el runtime no está sano.

## 4. Decisiones de unión y reducción

| Área actual | Unión propuesta | Qué desaparece / qué permanece |
| --- | --- | --- |
| `bootstrap::registration` + comprobaciones de `identity` | Una observación canónica del recurso en `identity.rs`, independiente de que Linux esté preparado. | Quitar descubrimiento de registros de bootstrap y comprobaciones paralelas. Conservar distinción entre ausencia, recurso propio parcial/preparado, conflicto y error de lectura. |
| `pending_target`, `target_owner`, `verify_target`, `verify_removed` | Consumir esa observación y una identidad abierta por fase; revalidar inmediatamente antes de la mutación. | Quitar aperturas completas de token/perfil realizadas solo para repetir la misma comprobación. No quitar revalidaciones entre consentimiento, instalación y borrado. |
| `node::WslSession` + `product_update::RuntimeHold` | Una primitiva de sesión en `identity.rs`, con dueños distintos para servicio y actualización conservadora. | Quitar implementación duplicada de cancelación/hilo/cierre. Conservar intención de parada; reset no mantiene vivo el nodo a destruir. Propagar fallos de apertura, no ocultarlos. |
| Fases de texto + banderas de transacción e inventario | Un vocabulario interno de operación y fase con invariantes explícitas. | Quitar decisiones duplicadas por combinaciones ambiguas; no crear otro gestor de estados. Mantener compatibilidad de recibos y bloqueo de resultados desconocidos. |
| Preparación inicial, aplicación de paquete y continuación | Un bootstrap y un transporte de payload verificado, invocados según la política observada. | Quitar ramas de preparación equivalentes; conservar hashes, identidad temprana, stdin y staging propio. Actualizar una instancia no importa otro rootfs. |
| Retirada MSI acoplada a excepciones del actualizador y PID del cliente | Evaluar una entrada MSI fija que invoque la misma retirada Rust bajo SYSTEM y el mismo lease, sin depender de un servicio Windows activo. | Potencialmente quitar el puente excepcional `permit_retire`/PID y su propagación por pipe/controller. Reutilizar ownership, exclusión, retirada de unidades y publicación; no otro worker ni una implementación alternativa de retirada. |

La última decisión **modifica un límite funcional vigente**: actualmente la
desinstalación necesita `GnXCore` en ejecución. Es una propuesta recomendada, no una
capacidad actual ni un fallback autorizado. Antes de implementarla hay que aprobar
ese cambio de contrato, resolver rollback con la intención previa y demostrar que
la entrada exige SYSTEM, lease, propiedad y exclusión de operaciones incompatibles.
Si no puede reutilizar exactamente las mismas funciones, no se añade una segunda
ruta de recuperación para hacerla funcionar.

El formato persistente no se cambia en el primer paso por conveniencia de los
tipos internos. Una transición posterior exige lector compatible y reglas para
recibos completos, cancelados e interrumpidos; nunca relabeling manual.

## 5. Presupuesto por frontera, no por cantidad de archivos

El núcleo inicial de revisión suma **3.629 líneas Rust**:

| Archivo | LOC base | Responsabilidad final |
| --- | ---: | --- |
| `windows/product_update.rs` | 1.014 | Coordinación de transacción/worker y handoff nativo; llamar a primitivas, no repetirlas. |
| `windows/identity.rs` | 788 | Observación/ejecución bajo identidad y sesión compartida. Puede recibir código movido de otros archivos. |
| `windows/bootstrap.rs` | 700 | Única preparación y aplicación del payload, sin descubrimiento de ownership duplicado. |
| `windows/recreation.rs` | 639 | Política destructiva, inventario/consentimiento y composición de retirada + bootstrap. |
| `windows/node.rs` | 488 | Lifecycle Linux y retirada conservadora común. |

`access.rs` mantiene publicación; `maintenance.rs`, el lease. `release_source.rs`
conserva descubrimiento HTTPS; `installer_package.rs`, validación/ownership MSI y
Burn. `console.rs` conserva la UX de secretos y queda fuera del refactor inicial.
No fusionar seguridad de instaladores con red ni mover estos bloques enteros a
`product_update.rs` para aparentar menos módulos.

**Meta orientativa, todavía no demostrada:** 300–500 líneas Rust netas menos
(aproximadamente 4–6% del Rust actual), sin crecer scripts/CI/instalador para
compensarlo. El primer mapa de llamadas debe justificar la estimación por bloques
concretos; si no lo permite, se reestima antes de implementar. No se promete volver
al tamaño de 0.0.22 conservando todas las funciones posteriores.

La aceptación exige reducción neta de Rust **y** del conjunto código/automatización,
sin minificar, eliminar explicaciones útiles, sustituir código por instrucciones
manuales, añadir dependencias o módulos de runtime, ni retirar funciones sin permiso.
Cada unión debe identificar llamadores eliminados, el dueño único y las garantías
que conserva. Un cambio de nombres o ubicación sin reducción de caminos no cuenta.

## 6. Higiene del repositorio, separada del refactor

Hay **10 intermedios seguidos en `installer/obj/`**: restauración NuGet, listas de
build, cachés y dos copias de `utilca.dll-1`; 812.564 bytes en total y 234 líneas de
texto. Proceden de commits anteriores al crecimiento 0.0.23; no son fuentes runtime.
`.gitignore` excluye `installer/bundle/obj/`, pero no `installer/obj/`.

Propuesta independiente: tras confirmar su regeneración mediante el build canónico,
sacarlos únicamente del índice y añadir la exclusión que falta. No borrado recursivo
del host, reescritura de historia ni eliminación de `releases/v*/gnx.exe`/changelogs.
Estos 234 renglones **no cuentan** como reducción de código del producto.

Descargas locales, WSL de desarrollo y cachés nativas tienen ownership y políticas
distintas. Su limpieza necesita inventario y permiso propios; no entra en esta
propuesta ni en los totales de fuentes. No borrar cachés MSI/Burn para adelgazar Git.

## 7. Secuencia y señal de cierre

1. Aprobar alcance y mapa de llamadas: bloque que se une, dueño, delta estimado,
   recibos afectados y garantías. Decidir explícitamente la entrada MSI propuesta.
2. Unificar observación/identidad y sesión dentro de archivos existentes; después
   simplificar consumidores. No empezar por borrar funciones todavía llamadas.
3. Simplificar estado interno y composición de operaciones. Mantener formatos y
   entrada MSI actuales hasta aprobar su cambio específico y su compatibilidad.
4. Separar la higiene de archivos generados de los cambios funcionales. Presentar
   diff, LOC antes/después por categoría y llamadas duplicadas retiradas.
5. Formato, análisis estático y verificaciones existentes solo acreditan sus gates.
   No generar tests, mocks ni harnesses ni ejecutar operaciones mutantes con este MD.

El cierre funcional requiere evidencia nativa autorizada de actualización con datos
conservados, retirada conservadora y rollback/reinstalación por la vía MSI/Burn
elegida, y reset del recurso propio seguido del bootstrap. La retirada parcial no
exige nodo sano; conflicto, error de lectura, operación activa o resultado desconocido
siguen bloqueando. Si se aprueba el cambio MSI, debe acreditarse también sin servicio
Windows activo, sin un arranque implícito del nodo para poder desinstalarlo.

No basta SCM `Running`, un menú de desinstalación, un recibo completado o una prueba
Linux en otra identidad. No debilitar TLS/revocación, ACL, SID/GUID/BasePath, hashes,
exclusión o confirmación de pérdida para alcanzar la meta de líneas.

Esta propuesta no asigna una versión ni ordena un build. Si después se autoriza una
entrega binaria, seguir [packaging](packaging.md), conservar el CLI canónico y no
sobrescribir versiones publicadas. El resultado documental y la implementación,
entrega y aceptación se cierran por separado.
