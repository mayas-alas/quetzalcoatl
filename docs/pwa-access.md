# Acceso privado a GnX

## Qué se controla

La **máquina de servicios privados** es el LXC compartido; no es RustFS.
Dentro funcionan almacenamiento **RustFS**, identidad **Pocket ID** y red
**NetBird**. OAuth2 Proxy y Valkey funcionan junto al gateway, fuera del LXC.
El árbol de distribución es `payload/node/workloads/private-services/`.

El tag `gnx-rustfs`, las claves IPC/estado y las rutas de guest/datastore
heredadas se conservan como identificadores de compatibilidad. No implican
otro invitado ni autorizan mover datos, cambiar credenciales o reenrollar.

## Alta humana y autorización

1. Abrir `https://pocket.gnx`; una persona completa el alta y registra su propia
   passkey. El agente no escribe PIN, identidad personal ni claves de acceso.
2. El administrador decide explícitamente quién pertenece a **gnx-app-users**,
   desde la administración de grupos de Pocket ID. Crear la cuenta no autoriza
   automáticamente el acceso a la PWA. El instalador no asigna personas.
3. Abrir `https://app.gnx`; la redirección lleva a **Iniciar sesión en GnX App**
   de Pocket ID. Completar la passkey en Windows Hello/dispositivo personal.
4. Si se rechaza el acceso, revisar la pertenencia al grupo, no desactivar TLS,
   ni habilitar archivos anónimos, ni reutilizar las credenciales de NetBird.

En Windows instalado ya existe confianza de la CA privada y resolución local.
Otros clientes del tailnet necesitan el split DNS y confiar explícitamente en
la CA privada. No hay publicación pública/LAN ni una migración a NetBird.

## Cerrar sesión

Usar **Cerrar sesión de GnX**. Es una petición POST del mismo origen a `/logout`,
con destino fijo. El servidor elimina la sesión en Valkey; la PWA borra la vista
privada y se emiten cabeceras de limpieza del navegador. El worker antiguo se
retira; no hay shell offline autenticado ni caché de `status.json`.

- No equivale a cerrar la sesión global de Pocket ID, RustFS o NetBird.
- Si el proxy o la red fallan, la PWA no confirma falsamente la eliminación de
  sesión: no se debe interpretar un error o una pestaña cerrada como logout.
- La cookie es Secure/HttpOnly/host-only, TTL 15 minutos renovable; el proxy
  solicita refresco cada minuto. La prueba aislada demuestra revalidación de
  grupos al refrescar, **no mide todavía la revocación con Pocket real**.
- Un reinicio de Valkey invalida sus sesiones porque no se persisten.

## Apagado y recuperación

Apagar la máquina compartida detiene RustFS, Pocket ID y NetBird. No será posible
un nuevo login de la PWA mientras Pocket ID esté apagado. En una terminal nativa
administrativa del host:

```powershell
gnx workload private-services start
# Apagado deliberado de todos esos servicios:
gnx workload private-services stop --confirm
```

Los comandos y URI antiguos `rustfs` siguen siendo aliases de compatibilidad.
Esto no es una API privilegiada en JavaScript ni un control plane localhost.
La consola propia de RustFS conserva su autenticación: el login de la PWA no
agrega un file manager, rutas arbitrarias de filesystem ni credenciales S3 a JS.

## Evidencia y límites

Ver `docs/validation/v0.1.71/README.md` y las notas del candidato. Los tests del
issuer aislado utilizan HTTPS, JWT firmados y las imágenes reales, pero no crean
usuarios en Pocket ni sustituyen una passkey personal. Capturas del login o del
PIN prueban la pantalla alcanzada, no un acceso autorizado ni un logout real.
