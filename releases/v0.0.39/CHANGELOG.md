# v0.0.39 — Infraestructura Kubernetes y observabilidad de red

Añade `gnx deploy --stack kluster`: preparación de la WSL dedicada y despliegue
controlado de un LXC propio con K3s, registro OCI con TLS, servicio HTTP,
comprobaciones de disponibilidad y observabilidad de red basada en Kubeshark.
Los recursos se organizan por función: `service-health`, `http-probe`,
`network-console`, `traffic-analysis` y `packet-capture`.

Incluye el ajuste acotado de CAP_SETFCAP para Fedora, límites de memoria del
trazador, reconciliación de Helm que conserva el estado operativo del coordinador
y comprobaciones de salud. Mantiene los recibos, identidad y controles del
servicio; no adopta ni migra automáticamente los LXC manuales existentes.

Candidata para instalación y pruebas manuales. El empaquetado no acredita
aceptación del stack, captura efectiva ni integración del cliente gráfico en Agent.
