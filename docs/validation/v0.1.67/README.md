# v0.1.67: evidencia del nodo instalado

## Alcance verificado

Pruebas del coordinador sobre el **runtime administrado real**, no sólo un contenedor de desarrollo:

- Burn/MSI v0.1.67 construido e instalado: exit `0`.
- `gnx upgrade --confirm`: `NodeReady`, credenciales conservadas.
- `gnx status` posterior: sigue `NodeReady`.
- LXC RustFS existente: `running`, RustFS/Pocket ID/NetBird: `healthy`.
- Todos los checks publicados en [status.json](status.json) pasaron, incluyendo discovery HTTPS privado, dashboard, Management/Signal HTTP/2 gRPC, relay WebSocket y respuesta de binding STUN.
- Provider Pocket ID asociado al IdP integrado de NetBird, cliente confidencial y callback correctos.
- Firefox: CA confiable sin saltarse advertencias, UI real de Pocket ID, selector NetBird con **Continue with Pocket ID**, y redirección real a **Iniciar sesión en GnX NetBird** en Pocket ID.
- Rust: 66 tests. Python: 7 tests en memoria para ACL y parsing gRPC. Contratos estáticos y control de flujo LXC mock: aprobados. Advertencia de linker LNK4098 observada.

## Capturas

1. [Nodo operativo](node-ready.png).
2. [LXC/RustFS y Pocket ID operativos](workload-pocket-ready.png).
3. [Pocket ID y NetBird operativos en la PWA](access-services-ready.png).
4. [Pocket ID login](pocket-login.png).
5. [NetBird ofrece Pocket ID](netbird-pocket-provider.png).
6. [Flujo NetBird → Pocket ID](pocket-netbird-flow.png): se recortó el chrome del browser para no publicar el identificador efímero de interacción OAuth; el contenido de la página no se alteró.
7. [Alta personal disponible](pocket-setup.png): formulario vacío; no se inventó una identidad ni un passkey.

## No verificado / acción personal

Estas capturas **no prueban una autenticación completada**, privilegios administrativos de una persona ni tráfico WireGuard entre peers. No se fabricó Windows Hello, un passkey o una sesión autenticada.

La persona operadora debe completar `https://pocket.gnx/setup` con su identidad y passkey, y después acceder a `https://netbird.gnx` mediante Pocket ID. Sigue pendiente revisar el posible solapamiento de rangos NetBird/Tailscale antes de enrolar peers, probar un peer real y la conectividad STUN de clientes externos (requiere una regla UDP 3478 hacia el gateway para esos clientes concretos).

Esto valida una actualización del nodo existente, no una instalación limpia o un cold boot de Windows/LXC. No se apagó el guest ni sus workloads ajenos para obtener evidencia. Las identidades, CA y datos existentes no se recrearon.
