# v0.0.26 — Candidata

**Estado:** corrección de bootstrap y continuación explícita; aceptación real pendiente.

- El bootstrap incorpora la cuenta técnica al grupo Usuarios mediante su SID
  verificado (API nativa nivel 0), no mediante un nombre relativo. Mantiene como
  resultado válido la pertenencia ya existente; no oculta otros errores Windows
  ni crea o adopta otra cuenta para eludirlos.
- Si una recreación interrumpida ya retiró la WSL y su estado, `update --force`
  puede ofrecer únicamente la preparación pendiente desde el payload instalado.
  Requiere el recibo anterior, el mismo SID y perfil, ausencia comprobada del GUID,
  VHD y cualquier registro GnX, y ausencia de otro estado de nodo o directorio con
  contenido. Exige escribir `PREPARAR`; no ejecuta otro `--unregister` ni limpieza.
- Si hay WSL, disco o estado nuevo/parcial, no se adopta ni se borra como parte de
  esa continuación. Un error de acceso al registro no se interpreta como ausencia.
  La barrera de interrupción se conserva ante cancelación o fallo temprano y no
  se reintentan automáticamente operaciones nativas con resultado desconocido.
- La recreación de un nodo todavía presente mantiene inventario, confirmación
  `RECREAR` y controles de propiedad. La actualización sin fuerza no pierde datos.

Instalar esta versión no completa automáticamente una recreación interrumpida.
La continuación requiere la CLI instalada y su confirmación explícita; no un CLI
técnico portable ni la edición manual de recibos. No restaura datos eliminados.
