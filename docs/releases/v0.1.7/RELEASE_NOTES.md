# GnX Node v0.1.7

## Corrección

- Supervisa el servidor del Named Pipe durante toda la vida del servicio.
- Si el pipe falla al arrancar durante una desinstalación/reinstalación o termina inesperadamente, `GnXHostAgent` lo vuelve a crear automáticamente.
- Evita el estado falso donde SCM muestra el servicio como `Running` pero `gnx status` informa que está indisponible.
- Conserva las correcciones de desconexión de clientes y serialización de la identidad WSL de v0.1.5/v0.1.6.

## Validación

- Pruebas Rust y contratos estáticos completos.
- Build y verificación de MSI/Burn.
- Actualización real a v0.1.7: el pipe respondió inmediatamente después de instalar, sin reiniciar manualmente el servicio.
- Prueba de recuperación: se ocupó deliberadamente `GnX.Platform.Control` al arrancar el servicio, se liberó la colisión y el supervisor recuperó el pipe automáticamente.
- `https://compute.gnx` respondió HTTP 200 tras la prueba.
