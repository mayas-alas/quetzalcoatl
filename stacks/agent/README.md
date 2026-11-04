# GnX Agent

Bytebot adaptado al LXC existente `kluster`, con Podman y Quadlets rootful. Configuración en `/stacks/agent`; scripts en `/stacks/agent/scripts`. Imágenes fijadas por digest en `images.json`, revisión upstream `3d37894ce07ef8d8b40adc7fd309ad96c2a71313`.

`bash /stacks/agent/scripts/deploy.run` instala los Quadlets y arranca la UI. Requiere el cliente de OmniRoute ya configurado en `/stacks/openui/secrets/apps.env`. PostgreSQL y escritorio conservan volúmenes nombrados; las contraseñas se generan una sola vez bajo `secrets/` con modo 0600. No elimina datos ni cambia el LXC a privilegiado.

Acceso: `https://agent.gnx`, usuario `demo` y contraseña privada de las demos. El gateway GnX conserva TLS y autenticación. UI, API y WebSockets pasan por el mismo origen; PostgreSQL, escritorio y puente LLM no publican puertos. El puente interno adapta el contrato LiteLLM de Bytebot a OmniRoute e inyecta la clave protegida; conserva herramientas, imágenes y streaming. Publica los combos de usuario identificados por OmniRoute y `auto/coding:free`, excluyendo las demás rutas `auto` y `auto/*`, y reenvía el modelo seleccionado; `LLM_MODEL` se usa como alternativa cuando no se indica ninguno.

Tailscale queda opcional y detenido: completar privadamente `/stacks/agent/secrets/tailscale.env` con `TS_AUTHKEY` y `TS_HOSTNAME`, después iniciar `agent-tailscale.service`. No reutiliza la identidad del nodo ni activa Funnel. La entrada privada GnX ya sirve el stack en este entorno.

Comprobación breve: `systemctl is-active agent-{postgres,desktop,llm,ui}.service agent.service`; abrir UI, verificar selector del combo y escritorio. El límite del escritorio es 2300 MiB; el LXC mantiene su aislamiento sin privilegios, aunque la imagen de escritorio usa el requisito `--privileged` dentro del LXC. No montar carpetas del host en él.

Al rotar la clave de cliente compartida, reiniciar `agent-llm.service` para recargar el archivo protegido. Cambiar el modelo dentro del combo de OmniRoute no requiere reiniciar las aplicaciones.
