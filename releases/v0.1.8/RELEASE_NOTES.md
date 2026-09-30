# GnX Node v0.1.8

## Limpieza completa

- Una desinstalación real elimina el servicio, archivos instalados, estado, CA, credenciales, datos runtime, cuenta/perfil `gnxnodesvc` y la distro/VHDX `gnx-node` registrada dentro de ese perfil.
- Elimina las entradas GnX de `hosts` y el port-proxy loopback HTTPS heredado.
- Una instalación genuinamente limpia ejecuta la misma limpieza antes de arrancar el servicio, retirando residuos dejados por versiones anteriores que conservaban el nodo.
- Los upgrades numerados conservan identidad, distro, runtime, CA y credenciales: la limpieza destructiva queda excluida mediante `UPGRADINGPRODUCTCODE` y `WIX_UPGRADE_DETECTED`.
- La credencial DPAPI del producto funciona como marcador de propiedad; no se elimina una cuenta ajena que coincida solamente por nombre.

## Aceptación real realizada

1. Se desinstaló v0.1.7 y se confirmó que dejó el nodo anterior, tal como hacía el contrato antiguo.
2. Se instaló v0.1.8 como instalación limpia y se comprobó la eliminación del perfil/distro anterior, cuenta, estado, `hosts` y port-proxy.
3. `gnx status` devolvió estado `New`; el Proxmox/LXC anterior dejó de ser accesible.
4. Se desinstaló v0.1.8 y se comprobó que servicio, archivos y todos los residuos permanecían ausentes.
5. Se reinstaló v0.1.8 y se dejó el host instalado en estado limpio para un alta nueva.

También pasaron las pruebas Rust, contratos estáticos y la construcción/verificación de MSI y Burn.
