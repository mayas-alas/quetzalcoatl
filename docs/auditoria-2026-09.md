# Auditoría 2026-09 — seguimiento de hallazgos

Reglas de trabajo:

- **Menos código, mejor.** Preferir borrar duplicados antes que añadir abstracciones.
- **No romper arquitectura.** Se conserva el modelo de `docs/arquitectura.md` y las
  fronteras de `ADR 0001`/`ADR 0002` (CLI → pipe → `GNXRuntime` → WSL → `gnx` Linux).
- **Frontera de producto.** Nada de orquestación de agentes dentro de `gnx`.
- Cada hallazgo tiene estado: `pendiente`, `hecho`, `no-cambio` (con motivo).

Gate de verificación tras cada bloque: `cargo test --locked` + `cargo clippy --locked
--all-targets -- -D warnings` en Windows, y `packaging/windows/build.ps1 -Validate`
(cruza la validación del binario Linux en contenedor) antes de cerrar el release.

## P0 — el repo no construye desde clone limpio

| ID | Hallazgo | Estado |
|---|---|---|
| A1 | `packaging/windows/runtime.lock.json` no está en Git, pero `build.ps1`, `validate.ps1` e `install-host.ps1` lo exigen | pendiente |
| A2 | `.gitattributes` no fija LF para `*.conf`, `*.timer`, `*.json`: con `core.autocrlf=true` salen CRLF y se incrustan al binario por `include_str!` | pendiente |

**Decisión D1 (A1).** Se versiona `runtime.lock.json`. Contiene únicamente el nombre de
la distro, un digest SHA-256 público y la URL canónica del rootfs Ubuntu 24.04. AGENTS.md
prohíbe tokens, claves privadas y URLs de *actualización*; GNX no tiene updater (ADR 0002
§5), así que esta URL es la de una imagen base pinneada en el build, no un endpoint de
actualización del producto. Si se prefiere otro criterio, la alternativa mínima es
ignorar el archivo y exigir que el empaquetado lo inyecte fuera de banda.

## P1 — legado

| ID | Hallazgo | Estado |
|---|---|---|
| B1 | `docs/operar.md` conserva tres referencias a Pi-hole (flujo, split DNS, paso 5 de instalación) cuando el split DNS lo sirve `gnx-dns` gestionado por `access` | pendiente |

## P2 — redundancias

| ID | Hallazgo | Estado |
|---|---|---|
| C1 | Taxonomía de acciones escrita tres veces: `cli.rs::command_args`, `broker.rs::action_code`, `broker.rs::code_action` | pendiente |
| C2 | `fn systemctl()` copiada en `access.rs`, `compute.rs` y `controller.rs` | pendiente |
| C3 | Digest de la imagen dnsmasq duplicada en `src/access.rs` y `runtime/access/gnx-dns.container` | pendiente |
| C4 | Literal `pki.gnx` en cuatro lugares (`access.rs` ×3, `controller.rs`) | pendiente |
| C5 | Prompt de auth key y pantalla alternativa de credenciales implementados dos veces (`access.rs`/`compute.rs` y `platform.rs`) | pendiente |
| C6 | Validación de SID duplicada e inconsistente: `account::valid_sid` (≤184) vs inline en `broker.rs` (sin tope) | pendiente |

## P3 — cosmética y acoplamiento

| ID | Hallazgo | Estado |
|---|---|---|
| E1 | `Error::Spawn` se reutiliza para lecto/escritura de pipe y de stdout; la etiqueta observable sale `PROCESS_START` | pendiente |
| E2 | `runtime::sync_config` repite el tope de tamaño que ya impone el framing del broker | pendiente |
| E3 | `enroll.sh` combina `trap … EXIT`, `rm -f` explícito y `trap - EXIT` | pendiente |
| E4 | Plantillas mixtas: `access` usa `@STATE@`, `compute`/`controller` fijan rutas absolutas | no-cambio |
| E5 | `Config::validate` exige rutas Linux aunque corra en Windows | no-cambio |
| E6 | `.gitignore` duplica `/target/` y `target` | pendiente |

**Motivo E4.** Los placeholders quedan sólo donde el valor es dinámico de verdad
(`@UPLINK@`, `@MTU@`, `@IP@`, `@STATE@` de access porque el bind-mount sale del config).
`compute` y `controller` montan rutas que el validador fija a un solo valor; convertirlas
en plantillas añadiría código sin añadir capacidad.

**Motivo E5.** El `gnx.toml` de Windows *es* el archivo que se instala en
`/etc/gnx/gnx.toml` dentro de la distro (`sync_config`), así que sus rutas deben ser
Linux por definición. Validar antes de enviar es deliberado (`cli.rs`). No hay acoplamiento
que corregir.
