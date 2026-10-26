# GnX Core v0.0.10

- Configura `gnxcoresvc` como cuenta local de servicio: permite logon de servicio
  y deniega inicio de sesión interactivo local y por Escritorio remoto.
- Oculta su entrada del selector de inicio/cambio de usuario. La cuenta continúa
  visible para administración; ocultarla no sustituye las restricciones de acceso.
- El proceso dedicado obtiene un token de servicio, sin fallback interactivo.
- Reconcilia la política de una instalación preparada tras comprobar ownership
  y SID, sin recrear la cuenta ni cambiar credenciales, perfil, WSL, CA o datos.

No cambia la base Linux ni los Quadlets de v0.0.9. La desinstalación conservadora
completa y la purga separada siguen pendientes; actualizar no ejecuta limpieza
del legado ni elimina datos.
