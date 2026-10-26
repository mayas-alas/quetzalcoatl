# GnX Core 0.0.13

## Cambios

- Conserva las mejoras HTTPS y mantenimiento nativo de 0.0.12, sin comando TLS.
- La retirada MSI usa el pipe administrativo y el servicio existentes: una sola
  frontera de identidad y serialización, sin otro ciclo de tokens/perfiles.
- Confirma la parada con el resultado nativo de systemctl, sin interpretar texto
  de presentación. Los fallos siguen bloqueando la retirada, no se ignoran.
- La reversión usa start/stop existentes según la intención preservada.
- El build elimina solo copias intermedias MSI/EXE idénticas a sus salidas
  verificadas y copias CAB cuya incorporación al MSI se comprueba; conserva todos
  los instaladores versionados y sus gates.

## Límites

- Recuperación mesh no aceptada: el gateway puede iniciar antes de `tailscale0`
  y conservar solo escucha NAT. Corrección en 0.0.14; HTTPS local no acredita mesh.

- Candidato. Comprobación manual en este host no acredita Windows limpio, reboot,
  expiración futura, aplicaciones LXC ni aceptación móvil remota.
- Retirada conservadora: cuenta, perfil, WSL, CA, credenciales y datos se conservan.
  Sin purga ni cambios de red, MTU, raíz o protecciones.
