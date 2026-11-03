# v0.0.34 — Onboarding inicial del stack privado

- Incluye la continuación conservadora `update --resume` del nodo parcial propio
  y la recuperación explícita de recibos de guests cuya ausencia se confirma en PVE.
  Ninguna de esas operaciones se ejecuta automáticamente al actualizar o arrancar.
- Espera la disponibilidad inicial del bus y el fin de arranque de systemd dentro
  de un único plazo de 120 segundos. Solo repite consultas de lectura ante el bus
  ausente o rechazando conexión; no repite creación, arranque ni preparación del LXC.
- Prepara grupos propios de Pocket ID para NetBird y RustFS mediante sus APIs;
  conserva miembros y grupos permitidos existentes, sin adoptar grupos por nombre.
- Identifica al único administrador humano inicial con passkey y liga su ID Pocket ID
  al sujeto y conector de NetBird. Solo ese operador recibe aprobación y rol Admin;
  no desactiva la aprobación global ni reemplaza al propietario técnico.
- Guía el alta humana y avisa de que NetBird requiere compartir correo. Distingue
  servicios desplegados de onboarding pendiente y permite continuar con la misma
  entrada en el guest ya desplegado, sin reinstalar aplicaciones ni reenviar secretos.
- Los clientes nuevos nacen restringidos por grupo. Al finalizar el onboarding,
  clientes propios antes abiertos pasan a restringidos; otros usuarios necesitan
  pertenecer a un grupo permitido. La CLI lo advierte antes de la finalización.
- Registra intención antes de las mutaciones nuevas y contrasta su resultado con
  lecturas nativas. Un resultado desconocido no se repite a ciegas; una creación de
  grupo sin ID devuelto o un token de preparación rechazado bloquean sin resetear.
- No crea identidades ni passkeys humanas, no recrea el nodo ni elimina guests.
  Tras completar el onboarding no reescribe los roles o membresías del operador.
- Corrige el enlace de código fuente de la app y la etiqueta de procedencia de
  construcción para utilizar el repositorio canónico `mayas-alas/quetzalcoatl`.
- La construcción no acredita instalación, login humano ni conectividad entre peers;
  la aceptación runtime de esta versión se comprueba por separado.
