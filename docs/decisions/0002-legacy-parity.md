# 0002 — Paridad con legacy: recuperar comportamiento, no portar código

- **Estado:** aceptada
- **Contexto:** rama `legacy` (snapshot 2026-08-31), AGENTS.md §5
- **Tags:** legacy, parity, scope, isolation

## Contexto

La rama `legacy` conserva el producto anterior: servicio Windows, cuenta aislada, tray, journal JSONL, Headscale, Docktail, OpenTofu y Podman Machine.

GNX 0.2 mantiene sólo tres capacidades de runtime (`access`, `compute`, `controller`) y un runtime Linux común. La revisión de Windows recupera una sola propiedad del legacy que sí sigue siendo útil: **la sesión cotidiana del operador no debe poseer la distro WSL ni Podman**.

## Matriz

| Capacidad legacy | GNX 0.2 | Decisión |
|---|---|---|
| Cuenta `gnx-runtime` | recuperada | **Sí.** Es la identidad propietaria del runtime Windows. |
| Servicio Windows | recuperado como `GNXRuntime` | **Sí, pero mínimo.** Sólo bootstrap + broker allowlisted. |
| Named Pipe | `\\.\pipe\GNX` | **Sí.** Única frontera CLI → runtime en Windows. |
| Tray | ausente | **No portar.** |
| Podman Machine Fedora | ausente | **No portar.** Podman corre nativo en Ubuntu WSL. |
| Journal JSONL | ausente | **No portar.** Runtime Linux usa systemd. |
| `gnx repair` dedicado | ausente | **No portar.** Reaplicar operaciones idempotentes. |
| Headscale | ausente | **Fuera del MVP actual.** |
| OpenTofu runner | ausente | **Fuera del MVP actual.** |
| Docktail | ausente | **Sustituido por la superficie actual.** |
| Split DNS `.gnx` | dnsmasq mínimo | **Mantener.** |
| CA `.gnx` opcional | presente | **Mantener.** |

## Decisión

1. **Recuperar el security boundary, no el runtime legacy.**
   `gnx-runtime` vuelve únicamente para poseer `GNXRuntime`, la distro WSL `GNX` y Podman Linux.

2. **El operador usa sólo la CLI.**

   ```text
   gnx.exe → named pipe → GNXRuntime → WSL GNX → gnx Linux
   ```

   `gnx.exe` no invoca `wsl.exe` directamente.

3. **El servicio no converge infraestructura por su cuenta.**
   No contiene scheduler, state machine de producto ni loops de reparación. Hace bootstrap del substrate y ejecuta únicamente requests allowlisted.

4. **No recuperar Podman Machine.**
   WSL ya proporciona el substrate Linux; añadir otra VM volvería a introducir una capa sin aportar al boundary de usuario.

5. **No recuperar tray, updater ni journal propio.**
   No son necesarios para ocultar el runtime de la sesión del operador.

## Consecuencias

- La distro WSL queda registrada en el perfil de `gnx-runtime`, no en el del operador.
- Podman existe únicamente dentro de esa distro.
- Windows vuelve a tener un servicio, pero su responsabilidad es estrecha y comprobable.
- El pipe no es un shell privilegiado: ocho opcodes cubren exactamente la CLI actual.
- Administradores locales y SYSTEM siguen siendo trust principals del host; el boundary no pretende protegerse de ellos.
- La arquitectura Linux de `access`, `compute` y `controller` permanece sin cambios.

## References

- `src/windows/account.rs` — identidad dedicada y derechos de logon.
- `src/windows/service.rs` — servicio `GNXRuntime`.
- `src/windows/broker.rs` — pipe y allowlist.
- `src/windows/runtime.rs` — distro `GNX` y bootstrap Linux.
- `docs/arquitectura.md` — modelo actualizado.
