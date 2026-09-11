# Auditoría del repositorio GNX — 2026-09-11

## Decisión

No conviene tirar el repositorio completo. Conviene conservar el contrato pequeño
de GNX y rehacer la composición de runtime y despliegue desde una base limpia.
El código actual es una base de contratos y pruebas, no todavía un producto
instalable y recuperable.

La recomendación es congelar `main` como referencia, crear una línea de trabajo
de ejecución limpia y avanzar por una sola topología Quadlet dentro de la distro
WSL aislada. El contrato objetivo es Windows host → `GNXRuntime` bajo
`.\gnx-runtime` → distro WSL `GNX` → binario Linux GNX inyectado y verificado
desde el instalador/EXE. La implementación actual mezcla esta ruta con restos de
la instalación Ubuntu anterior.

## Evidencia revisada

- Checkout limpio: `main` en `fe5fa98`, sincronizado con `origin/main`.
- `cargo test --locked --all-targets`: 16 pruebas ejecutadas, todas pasan; 1
  prueba local permanece ignorada.
- `cargo clippy --locked --all-targets -- -D warnings`: pasa.
- `doctor` secuencial en Windows: `READY/HOST_READY`, pero eso solo valida el
  host, no el runtime.
- `status` secuencial: `ACTION_REQUIRED/RUNTIME_NOT_READY`, con
  `ACCESS_ENROLLMENT_REQUIRED` y `SERVICE_NOT_ACTIVE`.
- Servicio `GNXRuntime`: instalado, automático y en ejecución bajo
  `.​\gnx-runtime`.
- `wsl --list --verbose`: solo aparece `Ubuntu-24.04`; no aparece la distribución
  aislada `GNX` requerida por el contrato.
- `C:\ProgramData\GNX` conserva directorios y reportes de instalaciones
  anteriores; `C:\Program Files\GNX` conserva binarios actuales y copias
  `.pre-032rc2`.
- No existe comando, binario o script `uninstall` en el repositorio.
- No existe evidencia actual de la distro aislada `GNX` operativa ni de una ruta
  remota G0–G6 después del reinicio.

## Madurez por área

| Área | Estado | Evaluación |
| --- | --- | --- |
| Contrato CLI/JSON | Parcialmente sólido | Las cuatro operaciones, estados y códigos tienen una forma coherente y pruebas de framing/paridad básica. |
| Validación de intención | Parcialmente sólida | Rechaza campos desconocidos, esquemas inválidos, nombres inválidos y varios orígenes inseguros. |
| Dominio y límites | Bueno como diseño | `domain` y `app` no importan adapters; la prueba arquitectónica pasa. |
| Reconciliación | Parcial | Hay lock, staging, promoción y restauración, pero faltan journal de crash, rollback completo, cleanup de fases y prueba de interrupción real. |
| Compute | Funcional en un entorno previo | Existe una comprobación autenticada del servicio y persistencia, pero depende de una instalación anterior y no está reproducible en la distro objetivo actual. |
| Access/DNS | Implementado, no aceptado | Tailscale y CoreDNS están cableados, pero enrollment, cliente no autorizado, split DNS y persistencia después de reboot no tienen evidencia actual. |
| Control/TLS | Implementado, no aceptado | Caddy y CA local existen en código; faltan instalación limpia, trust deliberado, SNI/Host negativos y prueba remota actual. |
| Quadlet | Incoherente | Los assets versionados están vacíos; Rust genera unidades en `/etc/containers/systemd` y además migra unidades systemd antiguas. Hay dos fuentes de verdad. |
| Windows | Riesgo alto | SCM, cuenta y pipe existen, pero falta distribución `GNX`, instalación transaccional, actualización, rollback, límites completos y prueba sin iniciar sesión. |
| Ciclo de vida | Incompleto | No hay uninstall, reinstall, upgrade autenticado ni limpieza segura de estado huérfano. |
| Release/supply chain | Insuficiente | Las imágenes tienen digest, pero `PinnedRelease::verify` solo valida sintaxis; no autentica firma ni procedencia del manifest. |
| Aceptación | No aceptada | `docs/release-validation.md` lo declara explícitamente y faltan pruebas requeridas de G0–G6. |

## Hoyos que bloquean usarlo como producto

1. **No hay una instalación reproducible desde cero.** El instalador rechaza
   estado existente, no hay uninstall y el bootstrap de WSL no es transaccional.
   Esto impide evaluar upgrade, reinstall y recuperación ante fallo.

2. **La topología operativa todavía no está cerrada.** El host conserva una
   Ubuntu anterior y el servicio Windows, pero no la distro aislada `GNX` que
   debe ser dueña del runtime. La limpieza debe eliminar la ambigüedad y dejar
   una sola ruta de ejecución mediante el binario Linux inyectado por Windows.

3. **Quadlet tiene dos autoridades.** `runtime/*/*.container` no contiene la
   configuración real; `src/adapter/linux.rs` la genera y escribe directamente.
   El cambio de una imagen o dependencia puede modificar código Rust y assets
   sin una única revisión declarativa del despliegue.

4. **El runtime Windows no coincide con la evidencia disponible.** El servicio
   está presente, pero la distro `GNX` no. Por tanto, `GNXRuntime` no prueba la
   ruta dedicada que exige el diseño.

5. **El release no está autenticado por productor.** Un digest correcto prueba
   integridad del archivo recibido; no prueba quién lo publicó. Falta manifest
   firmado, clave confiable embebida y verificación antes de instalar o aplicar.

6. **La transacción no es todavía crash-safe.** El estado registra fases, pero
   la restauración llama de nuevo a una reconciliación que puede reiniciar o
   cambiar servicios. No hay prueba de interrupción en cada fase ni rollback
   verificable de unidades, red, imágenes y certificados.

7. **La intención por defecto produce ruido falso.** `gnx.toml` publica
   `example.gnx` hacia una dirección privada no disponible y `status` reporta
   `UPSTREAM_UNREACHABLE` aunque no se haya solicitado una aplicación real.

8. **La superficie de nombres conserva historia.** Quedan
   `config/gnx.example.toml`, `docs/poc.md`, `poc031` en pruebas y comentarios,
   además de referencias de migración histórica. Esto confunde configuración
   operativa, aceptación y material archivado.

## Qué conservar

Conservar `Config`, `Report`, los cuatro comandos, el mapping de estados/exit
codes, la separación `domain/app/port/adapter`, la validación estricta de rutas,
el framing acotado del broker y las pruebas de no mutación/promoción. Son piezas
pequeñas y útiles.

Conservar también la regla de que `READY` depende de probes reales, la identidad
y datos persistentes sobreviven a reapply, y los secretos viajan por un canal
separado. Estas decisiones son mejores que el código histórico paralelo.

## Qué rehacer

Rehacer la instalación y el runtime Linux primero, con una sola autoridad:
assets Quadlet versionados y un adapter que solo los materialice con datos
variables. La autoridad de ejecución será la distro WSL `GNX`, alimentada por el
EXE/instalador de Windows bajo `GNXRuntime` y el usuario dedicado. La Ubuntu
existente queda como estado heredado de migración y se limpia después de capturar
la evidencia mínima necesaria.

LXC queda fuera del roadmap actual. Proxmox puede conservarse únicamente como
servicio de Compute si la decisión de release lo requiere; no se introduce una
capa adicional de provisión LXC.

Después implementar uninstall/reinstall seguro, release firmado, journal de
transacción y pruebas de recuperación. Windows debe ser un puente a esa misma
ruta Linux, y solo después debe cerrarse la instalación de la distribución
aislada y la prueba sin sesión del operador.

## Limpieza de taxonomía propuesta

- `config/gnx.example.toml` → `config/gnx.default.toml`.
- `docs/poc.md` → `docs/acceptance.md`; usar “candidate”, “acceptance” o
  “release” según el caso.
- Sustituir `poc031` por una instancia explícita como `gnx-audit-node` en tests,
  fixtures y documentación.
- Eliminar la ruta `example.gnx` del intent operativo por defecto; las rutas
  opcionales deben declararse solo cuando exista una aplicación real.
- Separar documentos históricos en un archivo o carpeta de archivo que no forme
  parte del contrato operativo.

## Orden de trabajo recomendado

1. Congelar esta auditoría y limpiar nombres/versiones de documentación.
2. Definir la limpieza de Ubuntu heredado y el dueño de cada estado persistente.
3. Construir/importar la distro WSL `GNX` y verificar que el binario Linux llega
   únicamente por el canal de bootstrap del EXE/servicio.
4. Construir una instalación limpia con Quadlet como única fuente.
5. Implementar `uninstall`, reinstall y rollback con estado protegido.
6. Hacer reales Compute, Access y Control en ese orden, con probes negativos.
7. Ejecutar G0–G5 en una instancia nueva y documentar evidencia sanitizada.
8. Cerrar el puente Windows y G6.

## Veredicto

El repositorio tiene madurez suficiente para rescatar contratos y decisiones,
pero no para desplegarlo como plataforma confiable después de un reinicio. El
camino de menor riesgo es un reinicio arquitectónico de la capa runtime dentro
del mismo proyecto, conservando el contrato probado y dejando una topología
única Windows/usuario dedicado/WSL/Quadlet con una operación completa de ciclo
de vida.
