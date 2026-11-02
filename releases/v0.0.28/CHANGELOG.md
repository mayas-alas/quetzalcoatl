# v0.0.28 — Candidata

**Estado:** candidata PoC; instalación y aceptación en host pendientes.

- `gnx deploy` sin argumentos selecciona el stack fijo `private-services`
  incorporado: Pocket ID, RustFS, NetBird y su dashboard. Mantiene
  `gnx deploy <carpeta>` para entradas administrativas explícitas. No despliega
  aplicaciones implícitamente al instalar ni al arrancar.
- El servicio prepara claves aleatorias propias, la configuración de NetBird y
  la CA pública del nodo. Conserva la entrada exacta en estado Windows protegido,
  ligada al SID/GUID WSL, antes de reservar un LXC. Repetir no rota secretos ni
  solicita otro guest para la misma entrada. No sustituye un stack ajeno o distinto;
  los recibos parciales y resultados desconocidos mantienen sus bloqueos.
- La CLI muestra etapas y estados de unidades en vivo por el pipe autenticado,
  sin imprimir configuración privada ni salida bruta de procesos. Desconectar
  el cliente no cancela el trabajo nativo. Journal y tareas PVE conservan el detalle.
- `enroll` y `password` comparten entrada interactiva con asteriscos y admiten
  retroceso y cancelación. La indicación de pegado distingue Ctrl+V de la acción
  Pegar/Ctrl+Shift+V de Windows Terminal. La validación de formato de la auth key
  no acredita vigencia ni autenticación con Tailscale. No se incorpora `rpassword`.
- Se conserva la propuesta documental de síntesis de lifecycle; no se presenta
  como un refactor implementado ni cambia la política de retirada del producto.

El stack requiere compute configurado con `gnx password` y acceso a los registros
de imágenes. El primer operador completa `https://pocket.gnx/setup` con su passkey
real. Compilación, empaquetado y publicación no acreditan disponibilidad HTTPS,
login humano, S3, conectividad entre peers ni aceptación del ciclo MSI/Burn.
