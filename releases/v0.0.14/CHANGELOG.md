# GnX Core 0.0.14

## Cambios

- Conserva mantenimiento HTTPS y retirada/reinstalación conservadora de 0.0.13.
- El Quadlet mesh usa `tailscale wait` nativo para esperar interfaz/IP antes de
  reconciliar DNS y escucha del gateway. La app local no depende de esa espera.
- Reconciliación de acceso serializada con flock y comparación antes de escribir
  o reiniciar; sin observer, polling propio, dominios, puertos o daemon nuevos.
- La app normal deja de mostrar una acción de importación de CA. Se conserva su
  endpoint público para administración y dispositivos que aún no la confíen.
- Sin comando TLS, cambios de CA, MTU ni credenciales por reiniciar/reinstalar.

## Límites

- Candidato. La comprobación en este host no acredita Windows limpio, reboot,
  expiración futura, despliegue LXC ni aceptación del navegador móvil remoto.
- Retirada conservadora, no purga: cuenta, perfil, WSL, CA y datos se conservan.
