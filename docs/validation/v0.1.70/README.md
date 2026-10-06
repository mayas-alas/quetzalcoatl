# v0.1.70 — pantalla de login, no sesión aceptada

![Pocket solicita el login para GnX App](pocket-app-login.png)

Firefox real, mediante computer-use: abrir `https://app.gnx` llevó a Pocket
**Iniciar sesión en GnX App**. Al pulsar Iniciar sesión apareció Windows Hello,
**Escribe tu PIN**. No se completó la interacción personal ni se escribió un PIN.

Captura real del candidato instalado v0.1.70. Recortada para omitir el chrome y
la URL de interacción OAuth; el contenido de la página no fue alterado. No hay
captura publicada de una persona ni del diálogo de PIN.

v0.1.70 recuperó NodeReady y las comprobaciones anónimas; se sustituyó por
[v0.1.71](../v0.1.71/README.md), que evita registrar el worker retirado otra vez
y agrega aliases/confirmaciones semánticos para servicios privados.
