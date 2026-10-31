# v0.0.20

- Comandos administrativos `update --check` y `update --apply`, con catálogo local
  explícito por digest para las cuatro imágenes del núcleo y las cuatro del stack
  privado registrado. Un catálogo revisado puede aportarse como archivo JSON sin
  reconstruir GnX. No descubre ni adopta automáticamente `latest`.
- Comparación local del catálogo aprobado al iniciar el nodo, sin descargas ni
  aplicación automática. Resultado en el Visor de eventos de `GnXCore`.
- Conserva ownership, configuración, datos y paradas explícitas; descarga primero
  las imágenes candidatas, aplica mediante Quadlets y registra resultados por
  componente. Un resultado interrumpido bloquea la repetición a ciegas. Compute
  requiere una ventana con guests detenidos; no detiene guests por iniciativa propia.
- Consola oficial RustFS v0.1.38 incorporada como recurso offline verificado por
  SHA-256 y servida por Caddy. Incluye el arreglo upstream #272 al refresco continuo
  para usuarios no administradores, sin ampliar sus permisos ni reconstruir RustFS.
- No actualiza el CLI/MSI ni los filesystems WSL/LXC; no revierte migraciones de
  datos. Compilación, empaquetado, instalación y aceptación se verifican por separado.
