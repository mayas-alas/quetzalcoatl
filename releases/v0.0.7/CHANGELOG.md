# GnX Core v0.0.7

- `enroll` prepara el resolver de malla: Pi-hole escucha solo en `tailscale0`,
  con direcciones de `app.gnx` y `compute.gnx`, sin dependencia de la malla
  para el acceso local.
- La configuración del resolver vive en `dns.env`; el arranque local no exige
  la interfaz de malla.
- Caddy limita sus listeners a las direcciones locales y mesh configuradas.
- Las instalaciones nuevas emiten certificados GnX Labs; el certificado
  público de la CA queda disponible en `/gnx-labs-root.crt`.
- `status` informa la fase real publicada por el nodo, no un mensaje genérico.

La comprobación del split DNS en el admin de Tailscale y la validación desde
otro dispositivo se informan por separado; compilar no las acredita.
