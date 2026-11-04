# v0.0.36 — Recuperación conservadora y stack OpenUI

- La actualización espera disponibilidad del pipe tras recuperación, además de SCM
  y applied-version, antes de aplicar componentes; conserva la intención de parada.
- update --resume reconcilia explícitamente una instalación verificada con componentes
  interrumpidos. Verifica versión MSI, payload, assets, ausencia de worker e identidad;
  no reinstala, elimina ni recrea WSL. Conserva el recibo en components_pending hasta
  completar la aplicación explícita del catálogo.
- app.gnx incorpora una portada HTML/CSS con accesos a las demos y API privada.
- DNS, hosts y hoja TLS incluyen f1.gnx, intelligent.gnx, omniroute.gnx y api.gnx,
  conservando la CA propia.
- stacks/openui contiene fuentes fijadas, overlays F1 para OmniRoute, Containerfiles,
  Quadlets y scripts para /stacks/openui en el LXC existente autorizado. No se instala
  automáticamente al iniciar el núcleo ni convierte gnx deploy en un builder.
- El historial F1 se conserva en el navegador; inference, proveedor, TLS y acceso del
  stack requieren comprobación runtime independiente del build.
