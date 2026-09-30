# GnX Node v0.1.10

## Corrección del alta limpia

- El keepalive ya no inicia WSL en un nodo nuevo antes de que provisioning copie `.wslconfig` e importe `gnx-node`.
- Evita la carrera observada donde `wsl.exe` de `gnxnodesvc` arrancaba mientras `.wslconfig` aún no existía y el alta terminaba con `ACCESS_DENIED`.
- El keepalive comienza únicamente después de que el nodo tenga una dirección privada inscrita.
- Los fallos futuros de copia de configuración se clasifican como `WSL_CONFIG_COPY_FAILED` en vez del genérico `ACCESS_DENIED`.
- `gnx setup` vuelve a solicitar directamente la contraseña one-shot cuando un nodo nuevo quedó bloqueado en las fases Windows/cuenta/CA.

## Validación

- Pruebas Rust: 57 aprobadas.
- Contratos estáticos de installer/runtime/red aprobados.
- MSI y Burn construidos y verificados.
- Upgrade real v0.1.8 → v0.1.9 y v0.1.9 → v0.1.10 preservó SID y estado; ambas acciones destructivas MSI fueron omitidas por sus condiciones de upgrade.
- Tras instalar v0.1.10 y esperar, no existe ningún proceso `wsl.exe` de `gnxnodesvc` en el nodo aún no provisionado y `.wslconfig` sigue reservado para el flujo de provisioning.

La contraseña Proxmox del intento fallido era one-shot y debe introducirse nuevamente con `gnx setup` o `gnx set password`.
