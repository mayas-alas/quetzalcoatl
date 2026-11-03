# v0.0.35 — Recuperación de acceso a la identidad del stack

## Añadido

- `gnx recovery`: crea un código de acceso de un solo uso para la cuenta
  operadora de Pocket ID mediante el subcomando nativo upstream
  (`/app/pocket-id one-time-access-token`), dentro del contenedor pocket-id del
  guest propio. La petición viaja por el pipe autenticado del servicio, el
  identificador de cuenta se ingresa sin eco y el enlace imprime solo en la
  consola del operador: el servicio no lo persiste ni lo escribe en journal.
  No borra cuentas, passkeys, guests ni datos; no cambia roles ni alrededor.
