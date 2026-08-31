# Quetzalcoatl Next — Propuesta final de arquitectura lean y soberana

> **Entrada histórica, no normativa.** Se conserva como contexto del greenfield.
> El contrato vigente está en `IMPLEMENTATION-TRACKER.md`,
> `docs/architecture.md` y `docs/build.md`; si difieren, esos documentos gobiernan.

**Estado:** propuesta final para aprobación de negocio  
**Naturaleza:** producto greenfield; no es una migración del producto 0.x  
**Línea propuesta:** Quetzalcoatl Next 1.x  
**Targets:** Windows x86_64 y Linux x86_64  
**Control plane de mesh:** Headscale operado fuera de GNX  
**Runtime de datos:** clientes Tailscale conectados al Headscale indicado por el operador  
**Exposición declarativa:** Docktail, condicionada a compatibilidad demostrada con Headscale  
**Fecha de la propuesta:** 2026-08-28

---

## 0. Decisión ejecutiva

Quetzalcoatl Next se construirá desde cero como un producto pequeño, explícito y
soberano. El producto anterior permanece congelado en su propia línea y se utiliza
únicamente como fuente de conocimiento técnico.

No se portarán automáticamente:

- schemas;
- instaladores;
- manifests;
- payload generations;
- contratos IPC;
- reconciliadores;
- taxonomías;
- abstracciones;
- nombres internos;
- compatibilidad de upgrade.

Una pieza anterior sólo podrá incorporarse cuando:

1. resuelva una necesidad del nuevo producto;
2. encaje sin adaptadores transitorios;
3. pase los tests del contrato nuevo;
4. reduzca riesgo o trabajo;
5. no arrastre compatibilidad histórica.

La arquitectura objetivo es:

```text
Windows x86_64                       Linux x86_64
      │                                   │
      ├─ preparación Windows              ├─ preflight Linux
      │                                   │
      └──────── Podman Machine quetzalcoatl ────────┐
                                                     │
                                                  systemd
                    ┌────────────────────────────────┼───────────────┐
                    │                                │               │
              tailscaled                         Docktail        Proxmox
                    │                             Quadlet         Quadlet
                    │                                │               │
                    └──── Headscale controller ──────┘          OpenTofu
                                                                     │
                                                                     ▼
                                                                    LXC
                                                                     │
                                                                  systemd
                    ┌────────────────────────────────────────────────┼─────────┐
                    │                                                │         │
              tailscaled                                      Docktail     Podman
                    │                                          Quadlet         │
                    └──────── Headscale controller ───────────────┘       workload
                                                                          Quadlets
```

Headscale es el **control plane** de la red. No es un proxy obligatorio para todo
el tráfico. Los clientes intercambian identidad, claves, política y mapa de red con
Headscale; el tráfico de datos debe viajar directamente entre peers cuando sea
posible y utilizar un relay autorizado sólo cuando la conectividad directa falle.

GNX no utilizará los servidores de control de Tailscale Inc. y no realizará fallback
silencioso hacia ellos.

---

## 1. Problema de negocio

El producto anterior acumuló responsabilidades de instalación, compatibilidad,
payloads, schemas, múltiples superficies de configuración y mecanismos de
reconciliación que elevan:

- el costo de cambio;
- la superficie de auditoría;
- el tiempo de entrega;
- el riesgo de regresión;
- la cantidad de estados imposibles de reproducir;
- la dependencia de conocimiento histórico.

Negocio necesita un producto que:

- pueda entenderse de extremo a extremo;
- tenga una sola experiencia operativa;
- utilice infraestructura soberana;
- evite dependencias SaaS obligatorias;
- converja de manera determinista;
- sea recuperable;
- permita agregar workloads sin crear un framework;
- tenga costos de mantenimiento proporcionales al alcance real.

---

## 2. Goal del producto

Quetzalcoatl Next hace únicamente lo siguiente:

1. prepara el host soportado;
2. crea o reutiliza una Podman Machine llamada `quetzalcoatl`;
3. instala y converge un runtime Linux común;
4. conecta el runtime al Headscale indicado por el operador;
5. ejecuta Tailscale como servicio de sistema;
6. ejecuta Docktail, Proxmox (https://github.com/dockur/proxmox) y workloads mediante Quadlets;
7. ejecuta OpenTofu como operación one-shot;
8. crea LXC con límites cerrados;
9. instala Podman, Tailscale y Docktail dentro de cada LXC administrado;
10. expone workloads por la mesh soberana;
11. reporta estado y diagnóstico accionables;
12. actualiza y repara su propia instalación;
13. mantiene el mismo runtime lógico en Windows y Linux.

El resultado debe ser:

- pequeño;
- Rust-first;
- modular;
- idempotente;
- auditable;
- soberano;
- seguro por diseño;
- explícito ante fallos;
- sin compatibilidad accidental.

---

## 3. Principios rectores

### 3.1 Greenfield real

La línea 1.x no lee ni modifica estado de la línea anterior.

La convivencia de 0.x y 1.x en el mismo host queda fuera de scope. Si GNX Next
detecta una instalación, una Podman Machine o recursos reservados por 0.x, falla
con `LEGACY_CONFLICT` antes de mutar. El operador debe usar otro host o retirar la
instalación anterior mediante su propio procedimiento. GNX Next no adopta, renombra
ni destruye recursos legacy.

### 3.2 Una experiencia, no una capa por concepto

La experiencia pública se concentra en `gnx`.

No se crean comandos, crates, managers o providers “por si después”.

### 3.3 Mismo runtime después de host prep

Windows y Linux pueden preparar el host de forma distinta. Desde la Podman Machine,
el contrato operativo es el mismo.

### 3.4 Soberanía de control

GNX conserva exactamente el controller HTTPS configurado y no introduce fallbacks
silenciosos.

### 3.5 Declarativo donde aporta valor

- systemd gobierna lifecycle;
- Quadlets declaran contenedores;
- OpenTofu declara infraestructura;
- Docktail observa labels y reconcilia exposición;
- GNX orquesta el orden y verifica resultados.

### 3.6 Operaciones cerradas

Los usuarios expresan intención, no comandos arbitrarios.

### 3.7 Fallo cerrado

Si un componente no cumple su contrato, GNX se detiene y explica la causa.

### 3.8 Sin big-bang interno

El producto es greenfield, pero su implementación se valida por recorridos
verticales pequeños. “Desde cero” no significa “sin gates”.

---

## 4. Definiciones normativas

| Término | Definición |
|---|---|
| GNX | Binario y producto Quetzalcoatl Next. |
| Host | Sistema Windows o Linux donde el operador ejecuta GNX. |
| Runtime común | Podman Machine `quetzalcoatl` y sus unidades systemd. |
| Headscale | Control server soberano, externo a GNX. |
| Controller URL | URL HTTPS canónica entregada por el operador para registrar todos los clientes. |
| Mesh | Red WireGuard coordinada por Headscale y utilizada por clientes Tailscale. |
| DERP/relay | Camino de datos de respaldo cuando no existe conectividad directa. |
| Docktail | Reconciliador que observa contenedores etiquetados y configura exposición Tailscale. |
| Celda | Una frontera de runtime con su propio socket Podman y su propio `tailscaled`. |
| LXC | Contenedor de sistema creado por OpenTofu en Proxmox. |
| Workload | Servicio de producto ejecutado mediante Quadlet dentro de una celda. |
| Convergencia | Operación idempotente que lleva el estado observado al estado deseado. |
| Pre-auth key | Credencial Headscale acotada utilizada para registrar una identidad no humana. |

Los términos “Tailnet” y “Tailscale SaaS” no se utilizarán como sinónimos de la
mesh Headscale.

---

## 5. Decisiones duras

1. El nuevo producto no migra desde 0.x.
2. El runtime soporta un único backend de control de mesh: Headscale.
3. El controller URL es obligatorio.
4. No existe fallback al control plane de Tailscale Inc.
5. Cada frontera Podman tiene su propio Docktail.
6. Cada celda administrada tiene su propio cliente Tailscale.
7. OpenTofu no administra Windows, WSL ni Podman Machine.
8. OpenTofu no utiliza provisioners.
9. systemd es la autoridad de lifecycle Linux.
10. Los contenedores se declaran con Quadlets.
11. Docker Engine y Docker Compose no forman parte del producto.
12. GNX no expone shell remoto ni argv arbitrario.
13. Las imágenes OCI se despliegan por digest.
14. Los secretos no se almacenan en configuración declarativa.
15. El build de desarrollo puede ser unsigned; un release distribuible debe tener
    integridad y firma de plataforma apropiadas.
16. Headscale no es instalado ni administrado por GNX.
17. GNX no hace fork silencioso de Headscale, Docktail ni Tailscale.

---

## 6. Artefactos oficiales

### 6.1 Windows

```text
gnx-windows-x86_64.exe
```

Un único ejecutable contiene:

- CLI;
- instalación;
- reparación;
- actualización;
- desinstalación;
- Windows Service;
- reboot/resume;
- IPC local;
- tray icon;
- lógica de host prep;
- orquestación GNX.

Modos internos:

```text
gnx.exe __service
gnx.exe __tray
gnx.exe __resume
```

Los modos internos no son API pública.

### 6.2 Linux

```text
gnx-x86_64.AppImage
```

El AppImage contiene:

- CLI;
- lógica GNX;
- metadata de desktop cuando aplique;
- icono;
- AppRun mínimo.

No contiene:

- Podman;
- QEMU;
- imágenes OCI;
- Headscale;
- Tailscale;
- Docktail;
- OpenTofu;
- Proxmox.

### 6.3 Metadatos de release

```text
dist/
├─ gnx-windows-x86_64.exe
├─ gnx-x86_64.AppImage
├─ SHA256SUMS
├─ release.json
└─ THIRD_PARTY_NOTICES.md
```

`release.json` fija:

- versión GNX;
- commit;
- target;
- fecha reproducible;
- SHA-256;
- identidad de firma cuando aplique;
- versiones soportadas de dependencias;
- versión mínima de configuración;
- versión mínima del state.

---

## 7. Scope incluido

- CLI `gnx`;
- instalación y mantenimiento Windows;
- Windows Service nativo;
- tray Windows nativo;
- IPC Named Pipe;
- identidad dedicada del servicio;
- WSL cuando el provider Podman lo requiera;
- descarga verificada de dependencias;
- Podman Machine `quetzalcoatl`;
- Linux AppImage;
- runtime Linux común;
- Tailscale client y `tailscaled`;
- controller URL Headscale;
- enrolamiento con pre-auth keys;
- Docktail por celda;
- OpenTofu one-shot;
- Proxmox en Quadlet (https://github.com/dockur/proxmox);
- creación cerrada de LXC;
- Podman dentro de LXC;
- Quadlets dentro de LXC;
- workloads oficiales;
- estado;
- health;
- logs redactados;
- diagnóstico;
- update;
- repair;
- uninstall no destructivo de datos;
- verificación de supply chain;
- aceptación Windows y Linux.

---

## 8. Fuera de scope

- migración desde Quetzalcoatl 0.x;
- adopción de máquinas o LXC legacy;
- Tailscale SaaS como controller;
- despliegue de Headscale;
- administración general de Headscale;
- UI administrativa de Headscale;
- forks privados de Headscale;
- implementación propia del protocolo Tailscale;
- Kubernetes;
- Talos;
- Docker Engine;
- Docker Compose;
- providers abstractos;
- hypervisores alternativos;
- marketplaces;
- plugins;
- scripts enviados por el usuario;
- HCL suministrado por repositorios de workloads;
- listeners Windows adicionales;
- localhost web UI;
- Tauri;
- shell remoto general;
- auto-update silencioso;
- exposición pública de workloads;
- producción antes de resolver el gate Docktail/Headscale.

---

## 9. Arquitectura de alto nivel

```text
                           Headscale externo
                      https://mesh.example.com
                                │
              coordinación, claves, policy, DNS, DERP map
                                │
                  ┌─────────────┴─────────────┐
                  │                           │
       Podman Machine quetzalcoatl       dispositivos cliente
                  │
      ┌───────────┼───────────────────────────────┐
      │           │                               │
 tailscaled   Docktail                       Proxmox
 systemd      Quadlet                        Quadlet
      │           │                               │
      └── socket ─┘                               │
                                                  │
                                               OpenTofu
                                                  │
                      ┌───────────────────────────┼────────────┐
                      │                           │            │
                   LXC A                       LXC B         LXC C
                      │                           │            │
           tailscaled + Docktail       tailscaled + Docktail  ...
                      │                           │
              workload Quadlets          workload Quadlets
```

La topología evita la falsa suposición de que un Docktail exterior puede observar
contenedores dentro de motores Podman interiores.

---

## 10. Headscale como control plane soberano

Headscale se entrega como un servicio externo administrado por el operador o por
un proveedor aprobado por negocio.

GNX recibe:

```toml
[mesh]
controller_url = "https://mesh.example.com"
```

Requisitos del controller:

- URL HTTPS absoluta;
- hostname DNS, no IP literal en producción;
- puerto 443 salvo perfil de laboratorio explícito;
- certificado confiable por el sistema administrado;
- sin credenciales embebidas en la URL;
- sin fragment ni query string;
- accesible desde cada celda;
- versión Headscale fijada en la matriz de compatibilidad;
- backups y disponibilidad fuera de la responsabilidad de GNX.

GNX utiliza conceptualmente:

```text
tailscale up --login-server https://mesh.example.com
```

La ejecución real también incluirá:

- hostname cerrado;
- tags autorizados;
- pre-auth key por canal secreto;
- flags fijados por GNX;
- ausencia de flags aportados por el usuario.

### 10.1 Sin fallback SaaS

Si el controller URL:

- no responde;
- presenta TLS inválido;
- no es compatible;
- rechaza registro;
- entrega policy inválida;

GNX falla con `MESH_CONTROLLER_UNAVAILABLE` o un error más específico.

No reemplaza la URL por un controller conocido.

### 10.2 Data plane

El objetivo es peer-to-peer WireGuard.

Headscale coordina:

- identidad;
- claves;
- asignación de direcciones;
- policy;
- DNS;
- routes;
- mapa de DERP/relay.

Headscale no debe describirse como un proxy permanente de todos los datos.

### 10.3 DERP y relay

El operador de Headscale decide:

- DERP embebido;
- DERP externo;
- relay regional;
- política de STUN;
- disponibilidad.

GNX:

- consume el mapa entregado;
- verifica conectividad;
- reporta si el enlace es directo o relayed;
- no despliega DERP;
- no modifica el controller.

---

## 11. Gate de compatibilidad Docktail + Headscale

### 11.1 Estado conocido

Docktail depende de:

- socket Docker-compatible;
- socket local de Tailscale;
- Tailscale Services;
- capacidad de definir o reconciliar services;
- credenciales API/OAuth cuando cree objetos de control plane.

A la fecha de esta propuesta, Headscale documenta Tailscale Services como una
feature faltante y su implementación permanece abierta.

Por tanto:

> Docktail + Headscale es una dirección aprobable, pero no una capacidad GA hasta
> pasar el gate `MESH-SVC-01`.

### 11.2 Gate MESH-SVC-01

La implementación de producto no puede declarar READY hasta demostrar, contra una
versión estable y fijada de Headscale:

1. registro de un host etiquetado;
2. creación de un service;
3. advertisement del endpoint;
4. aprobación automática o cerrada;
5. resolución DNS;
6. conectividad HTTPS;
7. eliminación o drain;
8. reconciliación después de restart;
9. rotación de credenciales;
10. denegación por policy;
11. ausencia de llamadas a Tailscale SaaS;
12. compatibilidad con el socket Podman.

### 11.3 Política ante fallo del gate

Si el gate no pasa:

- el release queda bloqueado;
- GNX no hace fallback al SaaS;
- GNX no simula READY;
- GNX no introduce un reconciliador paralelo oculto;
- GNX no mantiene un fork privado sin aprobación de negocio.

Negocio deberá decidir entre:

- esperar soporte upstream estable;
- financiar una contribución upstream;
- sustituir Docktail;
- cambiar el modelo de exposición.

La decisión no se toma dentro del código.

---

## 12. Modelo de celdas

Una celda es una frontera que posee:

- un socket Podman;
- un `tailscaled`;
- una identidad mesh;
- un Docktail;
- uno o más workloads;
- state y logs locales;
- policy tags propios.

Existen dos tipos:

### 12.1 Celda runtime

La Podman Machine contiene:

```text
systemd
├─ tailscaled.service
├─ podman.socket
├─ docktail.service
├─ proxmox.service
└─ gnx-opentofu.service
```

### 12.2 Celda workload

Cada LXC administrado contiene:

```text
systemd
├─ tailscaled.service
├─ podman.socket
├─ docktail.service
└─ <workload>.service
```

Cada Docktail observa únicamente el socket de su celda.

---

## 13. Modelo Windows

### 13.1 Usuario interactivo

El operador:

- descarga GNX;
- verifica su procedencia;
- abre el EXE de distribución sin argumentos;
- acepta elevación;
- utiliza CLI y tray;
- no posee la Podman Machine;
- no recibe secretos internos;
- no administra directamente Quadlets.

### 13.2 Identidad dedicada

GNX utiliza una identidad de servicio dedicada.

La implementación preferida es una identidad administrada por Windows que:

- no tenga contraseña gestionada por GNX;
- no permita inicio interactivo;
- tenga SID estable;
- posea sus directorios;
- posea la Podman Machine;
- reciba privilegios mínimos;
- pueda tener un perfil válido.

Si Podman/WSL exige una cuenta local convencional, el cambio requiere:

- prueba física;
- threat model;
- política de contraseña;
- ACL;
- recuperación;
- eliminación;
- documentación.

No se crea una cuenta local sólo por imitar el diseño anterior.

### 13.3 Orden de instalación

```text
preflight
  ↓
crear identidad
  ↓
crear/cargar perfil
  ↓
crear directorios y ACL
  ↓
instalar dependencias verificadas
  ↓
registrar servicio
  ↓
aplicar restricciones de logon
  ↓
registrar tray y ARP
  ↓
iniciar o solicitar reboot
```

---

## 14. Reboot y resume Windows

El reboot es una transición normal.

Estados de instalación:

```text
installing
reboot_required
resuming
installed
failed
```

Journal mínimo:

```json
{
  "schema": 1,
  "operation_id": "uuid",
  "operation": "install",
  "checkpoint": "wsl_enabled",
  "target_version": "1.0.0",
  "reboot_required": true,
  "last_error": null
}
```

Reglas:

- escritura atómica;
- ACL sólo para servicio y administradores;
- ningún secreto;
- operación idempotente;
- checkpoint monotónico;
- no reanudar otra versión;
- no repetir una mutación completada;
- conservar evidencia de la última falla;
- borrar el journal sólo al completar o cancelar de forma segura.

---

## 15. Windows Service

El propio binario Rust implementa el protocolo de Windows Service.

Responsabilidades:

- bootstrap después de reboot;
- propiedad de Podman Machine;
- convergencia solicitada;
- mantenimiento privilegiado;
- IPC;
- estado;
- health;
- shutdown;
- actualización coordinada.

No contiene:

- UI web;
- shell arbitrario;
- plugins;
- providers;
- HCL de usuario;
- scripts de usuario;
- fallback de runtime;
- credenciales Headscale administrativas.

---

## 16. IPC Windows

Transporte:

```text
Named Pipe local con ACL
```

Operaciones iniciales:

```text
Status
Init
Doctor
Repair
Shutdown
```

Reglas:

- framing versionado;
- tamaño máximo;
- timeout;
- tipos cerrados;
- separación read/mutate;
- impersonación para autorizar mutaciones;
- administradores para operaciones privilegiadas;
- no scripts;
- no argv;
- no paths arbitrarios;
- no secretos en respuestas;
- incompatibilidad de schema falla cerrada.

El schema puede evolucionar durante 1.x, pero no se preservan schemas de 0.x.

---

## 17. Tray Windows

El tray usa:

```text
gnx.exe __tray
```

Funciones:

- mostrar estado;
- mostrar versión;
- indicar working, ready, degraded o failed;
- abrir diagnóstico;
- iniciar `gnx init`;
- abrir URLs privadas validadas;
- salir.

No:

- ejecuta Podman;
- escribe configuración;
- almacena secretos;
- administra Headscale;
- implementa state propio.

---

## 18. Modelo Linux

GNX Linux se distribuye como AppImage.

Preflight mínimo:

- arquitectura x86_64;
- kernel soportado;
- systemd donde sea requerido;
- Podman soportado;
- `podman machine`;
- QEMU;
- KVM;
- virtualización anidada para Proxmox;
- FUSE para ejecución AppImage o modo de extracción documentado;
- espacio, RAM y CPU;
- HTTPS hacia Headscale y repositorios permitidos.

Flujo:

```text
AppImage
  ↓
preflight
  ↓
Podman Machine quetzalcoatl
  ↓
runtime común
  ↓
gnx init
```

No se promete “Linux genérico”.

La matriz de soporte debe nombrar:

- distribución;
- versión;
- kernel;
- Podman;
- provider de máquina;
- QEMU;
- AppImage/FUSE;
- estado probado.

---

## 19. Podman Machine común

Nombre canónico:

```text
quetzalcoatl
```

Propiedades:

- una por instalación;
- recursos fijados por perfil;
- imagen/versiones soportadas;
- systemd;
- cgroup v2;
- KVM;
- TUN;
- FUSE;
- filesystem persistente;
- SSH sólo como transporte interno cerrado;
- no acceso ordinario del usuario.

GNX no crea nombres alternativos si el nombre está ocupado.

Si existe una máquina incompatible:

```text
MACHINE_NAME_CONFLICT
```

No se adopta ni destruye automáticamente.

---

## 20. systemd y Quadlets

systemd es la autoridad de lifecycle.

Permitidos:

- `*.container`;
- `*.volume` cuando exista storage persistente real;
- unidades `*.service` pequeñas;
- timers sólo con necesidad demostrada.

No permitidos inicialmente:

- `*.pod`;
- `*.network` administrados por GNX;
- Compose;
- supervisores paralelos;
- scripts de polling infinitos.

Toda unidad define:

- dependencia;
- orden;
- restart policy;
- timeout;
- health;
- usuario;
- filesystem;
- límites;
- logging;
- cleanup.

---

## 21. Tailscale client

`tailscaled` corre como servicio del sistema Linux de cada celda.

Contrato:

- controller URL explícito;
- identidad etiquetada;
- state persistente;
- TUN disponible;
- SSH deshabilitado salvo requerimiento aprobado;
- routes cerradas;
- MagicDNS según capacidad Headscale;
- pre-auth key sólo durante enrolamiento;
- no contenedor Tailscale como diseño principal;
- no fallback SaaS.

### 21.1 Enrolamiento

```text
GNX
  │
  ├─ valida controller URL
  ├─ materializa pre-auth key en tmpfs 0400
  ├─ ejecuta tailscale up con login-server fijado
  ├─ valida identidad, tags y controller
  ├─ espera conectividad
  └─ elimina credencial temporal
```

### 21.2 Re-enrolamiento

Si el state Tailscale se pierde:

- GNX no reutiliza automáticamente una credencial expirada;
- solicita una nueva pre-auth key;
- no cambia controller URL;
- no conserva la identidad anterior como si siguiera vigente;
- informa el cambio de identidad.

---

## 22. Docktail

Docktail corre como Quadlet en cada celda con workloads observables.

Entradas:

- socket Podman Docker-compatible;
- socket local de Tailscale;
- labels cerradas;
- credencial de Services cuando la integración Headscale la requiera;
- controller compatible.

Montajes conceptuales:

```text
/run/podman/podman.sock  -> /var/run/docker.sock  read-only
/var/run/tailscale/      -> /var/run/tailscale/
```

Reglas:

- imagen por digest;
- versión en matriz;
- sin `latest`;
- socket limitado a la celda;
- SELinux probado;
- rootless preferido si la conectividad funciona;
- ningún secret en labels;
- ningún secret en environment si existe file-backed secret;
- logs redactados;
- drain antes de remover endpoints;
- health independiente del workload.

### 22.1 Labels permitidas

El producto define una lista cerrada:

```text
docktail.service.enable
docktail.service.name
docktail.service.port
docktail.service.direct
```

Los repositorios de servicio no inyectan labels arbitrarias.

Las labels se generan desde policy GNX validada.

### 22.2 Credenciales

La pre-auth key de enrolamiento no se reutiliza como credencial Docktail.

Cuando Headscale implemente la API requerida:

- se prefiere OAuth client-credentials;
- scope mínimo;
- un cliente por celda o dominio de blast radius;
- secreto file-backed;
- rotación independiente;
- revocación comprobada.

Una API key global all-access no es aceptable para release.

---

## 23. OpenTofu

OpenTofu es one-shot.

```text
gnx init
  │
  └─ gnx-opentofu.service
       ├─ init
       ├─ validate
       ├─ plan
       └─ apply
```

Responsabilidades:

- recursos Proxmox pertenecientes a GNX;
- LXC;
- storage y límites declarados;
- outputs de identidad;
- destrucción sólo con operación explícita.

Prohibido:

- provisioners;
- `local-exec`;
- `remote-exec`;
- scripts de repositorio de workload;
- comandos del usuario;
- secretos persistidos sin aceptación explícita;
- Windows/WSL/Podman Machine;
- administración de Headscale.

El bootstrap del guest pertenece a una operación GNX cerrada ejecutada después de
crear el recurso.

---

## 24. Proxmox

Proxmox se ejecuta como Quadlet dentro de la Podman Machine. https://github.com/dockur/proxmox

GNX controla:

- imagen por digest;
- devices;
- storage;
- health;
- endpoint;
- credenciales locales;
- lifecycle;
- creación de LXC;
- límites de VMID.

No existe:

- `HypervisorProvider`;
- provider seleccionable;
- argv remoto arbitrario;
- exposición pública.

La UI administrativa se expone por la mesh únicamente después de:

- health local;
- identidad Tailscale válida;
- policy válida;
- gate de exposición.

---

## 25. LXC

Cada LXC es una celda independiente.

Bootstrap:

1. OpenTofu crea el LXC.
2. GNX espera estado running.
3. GNX ejecuta un programa fijo por stdin.
4. El programa instala paquetes fijados.
5. Configura repositorios verificados.
6. Instala Podman.
7. Instala Tailscale client.
8. Instala Quadlets GNX.
9. Arranca `tailscaled`.
10. Registra contra el controller URL.
11. Arranca Podman socket.
12. Arranca Docktail.
13. Arranca workload.
14. Verifica health local.
15. Verifica health mesh.

Canal remoto:

```text
pct exec <vmid-conocido> -- /bin/sh -s
```

El script procede del release GNX y viaja por stdin acotado.

No se aceptan scripts aportados por el usuario.

---

## 26. Workloads

Los workloads son datos declarativos cerrados.

```text
workloads/
└─ <slug>/
   ├─ workload.toml
   ├─ <slug>.container
   └─ health.json
```

`workload.toml` define:

- slug;
- imagen OCI por digest;
- puerto local;
- health path;
- recursos;
- volumen;
- tag mesh;
- service name;
- dependencias;
- política de update.

No define:

- HCL;
- shell;
- controller URL;
- pre-auth key;
- OAuth secret;
- VMID libre;
- host paths arbitrarios;
- capabilities arbitrarias.

Workloads iniciales candidatos:

- Garage;
- Forgejo;
- runner de Forgejo Actions;
- FreeLLMAPI;
- OmniRoute.

Cada inclusión requiere una necesidad de negocio y acceptance propia.

---

## 27. Configuración

Una ubicación canónica por plataforma:

```text
Windows: %ProgramData%\QuetzalcoatlNext\config.toml
Linux:   /etc/quetzalcoatl-next/config.toml
```

Ejemplo:

```toml
schema = 1

[mesh]
controller_url = "https://mesh.example.com"
expected_domain = "mesh.internal"

[runtime]
machine_name = "quetzalcoatl"
profile = "standard"

[services]
garage = true
forgejo = true
runner = false
```

La configuración:

- no contiene secretos;
- se valida completa antes de activar;
- se escribe atómicamente;
- tiene owner y ACL;
- rechaza campos desconocidos;
- no usa environment como segunda fuente de verdad;
- no cambia automáticamente controller URL;
- separa desired state de observed state.

---

## 28. Secretos

Clases:

| Secreto | Propósito | Persistencia |
|---|---|---|
| Headscale pre-auth key | Enrolar una celda | Sólo durante enrolamiento, salvo flujo aprobado de reparación. |
| Docktail OAuth secret | Administrar Services | Protegido y file-backed; independiente por blast radius. |
| Proxmox credential | Operación local GNX | Store protegido. |
| Workload secret | Aplicación | Store propio; nunca en manifiesto. |

### 28.1 Windows

- DPAPI o mecanismo del sistema ligado a identidad de servicio;
- ACL servicio + administradores;
- entropy separada por clase;
- atomic write;
- zeroization en memoria cuando sea viable.

### 28.2 Linux

- tmpfs para secretos transitorios;
- archivos `0400` o `0600`;
- directorios `0700`;
- systemd credentials cuando aplique;
- file-backed secrets;
- cleanup verificado.

### 28.3 Prohibiciones

Nunca en:

- argv;
- URLs;
- logs;
- state;
- Quadlets versionados;
- labels;
- OpenTofu state;
- OCI images;
- repositorio;
- crash reports;
- nombres de archivo derivados del valor.

---

## 29. Estado y journal

Configuración, secretos, estado observado y journal son objetos distintos.

### 29.1 Estado operativo

```json
{
  "schema": 1,
  "product_version": "1.0.0",
  "stage": "ready",
  "machine": "ready",
  "mesh": "ready",
  "controller_url_fingerprint": "sha256:...",
  "proxmox": "ready",
  "infra": "ready",
  "workloads": {
    "forgejo": "ready"
  },
  "last_success_utc": "2026-08-28T00:00:00Z",
  "last_error": null
}
```

Estados públicos:

```text
pending
working
ready
degraded
failed
```

`degraded` requiere:

- servicio principal disponible;
- dependencia secundaria afectada;
- diagnóstico explícito;
- ninguna falsificación de READY.

### 29.2 Estado de identidad

GNX conserva únicamente identificadores no secretos necesarios para:

- detectar drift;
- reconocer una celda;
- evitar duplicados;
- reportar re-enrolamiento.

### 29.3 Journal

El journal existe sólo para operaciones mutables:

- install;
- init;
- repair;
- update;
- uninstall.

No se convierte en historial infinito.

---

## 30. CLI pública

```text
gnx init
gnx status
gnx status --json
gnx doctor
gnx repair
gnx update
gnx uninstall
gnx version
gnx --version
```

Justificación:

- `install`: lifecycle Windows;
- `init`: convergencia funcional;
- `status`: observación;
- `doctor`: diagnóstico sin mutación;
- `repair`: reconvergencia explícita;
- `update`: cambio de release;
- `uninstall`: lifecycle completo;
- `version`: soporte.

No existen comandos separados para:

```text
configure mesh
configure runtime
configure platform
configure service
```

`gnx init` recibe o construye una sola configuración.

---

## 31. gnx init

```text
gnx init
  │
  ├─ parse + validate config
  ├─ host preflight
  ├─ dependency lock
  ├─ Podman Machine ready
  ├─ runtime files active
  ├─ tailscaled ready
  ├─ Headscale controller verified
  ├─ runtime identity enrolled
  ├─ Docktail compatibility gate
  ├─ Proxmox ready
  ├─ OpenTofu apply
  ├─ LXC cells ready
  ├─ cell identities enrolled
  ├─ Docktail cells ready
  ├─ workload Quadlets healthy
  ├─ mesh exposure healthy
  └─ READY
```

Propiedades:

- idempotente;
- resumible;
- sin fallback;
- sin prompts cuando se entrega config completa;
- prompts sólo en terminal interactiva;
- secrets por stdin;
- dry-run opcional sólo si representa fielmente el cambio;
- una operación exclusiva por instalación.

---

## 32. Status y health

`gnx status` responde:

- versión;
- stage;
- host;
- Podman Machine;
- controller URL redactado/canónico;
- conectividad control plane;
- peer path direct/relay;
- Tailscale identity;
- Proxmox;
- OpenTofu;
- LXC;
- Docktail por celda;
- workload;
- private URL;
- último error.

`--json` es schema cerrado.

No reporta:

- keys;
- tokens;
- passwords;
- stdout completo;
- paths privados innecesarios;
- configuration completa.

---

## 33. Ejecución de procesos

Una utilidad Rust central implementa:

- executable fijo;
- argv estructurado;
- cwd explícito;
- environment allowlist;
- stdin acotado;
- stdout/stderr acotados;
- timeout;
- kill + reap;
- exit code;
- redacción;
- métricas de duración.

No:

```text
sh -c
bash -c
cmd /c
powershell -Command <texto variable>
```

Los scripts fijos se ejecutan mediante:

```text
/bin/sh -s
python3 -
```

con programa repository-owned por stdin.

---

## 34. Errores y diagnóstico

Formato conceptual:

```json
{
  "code": "MESH_CONTROLLER_TLS_INVALID",
  "component": "mesh",
  "operation": "controller_preflight",
  "message": "El certificado del controller no es confiable.",
  "action": "Instale la CA aprobada o corrija controller_url.",
  "retryable": false
}
```

Familias:

- `HOST_*`;
- `INSTALL_*`;
- `MACHINE_*`;
- `MESH_*`;
- `DOCKTAIL_*`;
- `PROXMOX_*`;
- `TOFU_*`;
- `LXC_*`;
- `WORKLOAD_*`;
- `UPDATE_*`.

`gnx doctor`:

- no muta;
- verifica dependencias;
- verifica TLS;
- verifica sockets;
- verifica versions;
- verifica systemd;
- verifica storage;
- produce un reporte redactado.

---

## 35. Descarga y supply chain

GNX no embebe dependencias pesadas.

Cada dependencia tiene lock:

```toml
id = "podman"
version = "x.y.z"
url = "https://..."
sha256 = "..."
signature = "required"
publisher = "..."
```

Flujo:

```text
resolver lock
  ↓
descargar a staging
  ↓
verificar TLS
  ↓
verificar tamaño
  ↓
verificar SHA-256
  ↓
verificar firma/publisher cuando exista
  ↓
activar atómicamente
  ↓
eliminar staging
```

Prohibido:

- resolver “latest”;
- seguir un mirror no permitido;
- ejecutar antes de verificar;
- conservar descarga corrupta;
- cambiar versión por disponibilidad;
- descargar desde Tailscale SaaS como fallback de control.

---

## 36. Versiones y compatibilidad

La matriz fija:

- GNX;
- Windows;
- Linux;
- Podman;
- Podman Machine image;
- Tailscale client;
- Headscale;
- Docktail;
- OpenTofu;
- Proxmox image;
- providers OpenTofu;
- imágenes workload.

Reglas:

- Headscale `main` no es target;
- prereleases sólo en laboratorio;
- Docktail `latest` está prohibido;
- cliente Tailscale debe ser compatible con la versión Headscale fijada;
- una actualización de Headscale requiere rerun del gate MESH-SVC-01;
- incompatibilidad detiene init/update.

---

## 37. Build Windows

```powershell
.\scripts\build-windows.ps1
```

Salida de desarrollo:

```text
dist/gnx-windows-x86_64.exe
```

El build:

- compila release;
- fija metadata;
- genera checksum;
- no descarga payloads externos;
- no incorpora claves privadas;
- permite build local unsigned.

El release QA:

- se firma con identidad QA;
- no se presenta como producción;
- no instala trust fuera de un flujo explícitamente aprobado.

Un release público futuro:

- requiere certificado confiable;
- timestamp;
- verificación Authenticode;
- evidencia de Smart App Control/SmartScreen;
- proceso separado del build de desarrollo.

Lean no significa unsigned.

---

## 38. Build Linux

```bash
./scripts/build-appimage.sh
```

Salida:

```text
dist/gnx-x86_64.AppImage
```

El build:

- usa runtime base fijado;
- genera AppRun;
- incluye icono y desktop metadata;
- genera SHA-256;
- registra dependencias dinámicas;
- verifica ejecución con FUSE;
- verifica `--appimage-extract-and-run` o alternativa documentada;
- no incluye Podman, Tailscale ni imágenes.

---

## 39. Install, update, repair y uninstall

### Install

- crea identidad;
- directorios;
- ACL;
- servicio;
- tray;
- ARP;
- dependencias;
- journal;
- no ejecuta init sin configuración.

### Update

- descarga release;
- verifica;
- detiene operaciones;
- conserva config/state/secrets;
- sustituye binario atómicamente;
- aplica cambios de schema 1.x soportados;
- rollback si el nuevo servicio no inicia.

### Repair

- verifica archivos;
- repara ACL;
- re-registra servicio;
- reinstala dependencia fijada si falta;
- reconverge unidades;
- no rota identidad;
- no destruye datos.

### Uninstall

Por defecto elimina:

- binario;
- servicio;
- tray;
- registro ARP;
- archivos temporales.

Conserva:

- Podman Machine;
- LXC;
- volúmenes;
- workload data;
- backups;
- secretos persistentes necesarios para recovery.

La destrucción de datos requiere operación separada, explícita y confirmada.

---

## 40. Datos, backup y recovery

Dominios:

- configuración GNX;
- state GNX;
- secret store;
- Podman Machine;
- Proxmox storage;
- OpenTofu state;
- LXC volumes;
- workload data;
- Headscale externo.

GNX define:

- owner;
- ruta;
- backup;
- restore;
- integridad;
- retención;
- destrucción.

Headscale queda fuera del backup GNX. El operador debe garantizar:

- base de datos;
- claves privadas;
- policy;
- TLS;
- DERP configuration;
- restauración del mismo controller URL.

Sin esos elementos, una mesh soberana no es recuperable.

---

## 41. Seguridad por arquitectura

```text
interactive admin
      │
      │ Named Pipe cerrado
      ▼
GNX privileged service
      │
      │ operaciones cerradas
      ▼
Podman Machine
      │
      ├─ Headscale URL fijada
      ├─ Tailscale identity
      ├─ Docktail local
      └─ Proxmox/OpenTofu
```

Controles:

- separación de identidad;
- controller pin lógico;
- TLS;
- digest OCI;
- least privilege;
- secretos separados;
- no shell;
- no fallback;
- state sin secretos;
- locks de operación;
- logs acotados;
- ACL por tags;
- deny-by-default;
- actualizaciones explícitas.

### 41.1 Threats mínimos

Debe probarse:

- robo de pre-auth key;
- robo de OAuth secret;
- socket Podman comprometido;
- controller URL malicioso;
- DNS poisoning;
- certificado inválido;
- Headscale indisponible;
- DERP malicioso;
- workload con labels manipuladas;
- image digest cambiado;
- LXC comprometido;
- replay de journal;
- downgrade de versión.

---

## 42. Observabilidad

GNX registra:

- timestamp;
- operation ID;
- componente;
- etapa;
- duración;
- resultado;
- código de error.

No registra:

- secretos;
- argv sensible;
- configuración completa;
- bodies de API;
- tokens;
- passwords.

systemd/journald es la fuente runtime.

Windows Event Log es la fuente del servicio Windows.

No se introduce una plataforma de observabilidad.

---

## 43. Capacidad y perfiles

Perfiles iniciales:

| Perfil | Uso | CPU | RAM | Disco |
|---|---|---:|---:|---:|
| lab | evaluación | definido por benchmark | definido por benchmark | definido por benchmark |
| standard | workloads iniciales | definido por benchmark | definido por benchmark | definido por benchmark |

Los valores no se inventan en el contrato. Se fijan después del benchmark físico.

GNX rechaza un host insuficiente antes de mutar.

Cada workload declara límites.

---

## 44. Tests

### 44.1 Unit

- config parsing;
- URL canonicalization;
- prohibición de controller SaaS;
- command construction;
- state transitions;
- journal;
- redaction;
- secret cleanup;
- path validation;
- labels Docktail;
- workload manifest;
- IPC framing;
- error mapping.

### 44.2 Integration

```text
Podman socket
  ↓
Docktail
  ↓
labels
  ↓
Tailscale socket
  ↓
Headscale Services
  ↓
private endpoint
```

```text
gnx init
  ↓
machine
  ↓
runtime
  ↓
Proxmox
  ↓
OpenTofu
  ↓
LXC
  ↓
Podman + Quadlets
  ↓
healthy workload
```

### 44.3 Compatibilidad

Matriz real:

- Windows limpio;
- Linux soportado;
- Headscale fijado;
- Tailscale fijado;
- Docktail fijado;
- directo;
- DERP/relay;
- restart;
- controller outage;
- credential rotation.

### 44.4 Seguridad

- no secrets en logs;
- no secrets en argv;
- no SaaS endpoint;
- no mutable tags;
- no arbitrary commands;
- socket isolation;
- policy deny;
- tampered artifact rejection.

---

## 45. Acceptance Windows

```text
clean Windows host
  ↓
abrir el EXE sin argumentos
  ↓
reboot
  ↓
service identity ready
  ↓
Podman Machine owned by service
  ↓
gnx init
  ↓
Headscale registration
  ↓
Proxmox + LXC
  ↓
Docktail + workload
  ↓
private mesh health
  ↓
READY
```

También:

- update;
- rollback;
- repair;
- restart;
- uninstall no destructivo.

---

## 46. Acceptance Linux

```text
supported Linux host
  ↓
AppImage preflight
  ↓
Podman Machine qemu
  ↓
KVM/nested virtualization
  ↓
gnx init
  ↓
same runtime contract
  ↓
private mesh health
  ↓
READY
```

No se acepta Linux por inferencia de Windows.

---

## 47. Release gates

| Gate | Requisito |
|---|---|
| G-01 | Documento aprobado por negocio. |
| G-02 | Matriz de soporte aprobada. |
| G-03 | Windows service identity probada físicamente. |
| G-04 | Podman Machine Windows y Linux probada. |
| G-05 | Headscale controller URL y no-fallback probados. |
| G-06 / MESH-SVC-01 | Docktail + Headscale Services compatible de extremo a extremo. |
| G-07 | Secret model y rotación probados. |
| G-08 | OpenTofu sin provisioners. |
| G-09 | LXC Podman + Quadlets healthy. |
| G-10 | Update/repair/uninstall probados. |
| G-11 | Supply chain y firma QA probadas. |
| G-12 | No secrets / no SaaS / no arbitrary execution. |

Ningún gate se convierte en warning.

---

## 48. Taxonomía objetivo

```text
quetzalcoatl-next/
├─ Cargo.toml
├─ Cargo.lock
├─ rust-toolchain.toml
├─ build.rs
├─ README.md
├─ LICENSE
├─ assets/
│  ├─ quetzalcoatl.ico
│  └─ quetzalcoatl.png
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ cli/
│  │  ├─ install.rs
│  │  ├─ init.rs
│  │  ├─ status.rs
│  │  ├─ doctor.rs
│  │  ├─ repair.rs
│  │  ├─ update.rs
│  │  └─ uninstall.rs
│  ├─ host/
│  │  ├─ linux.rs
│  │  └─ windows/
│  │     ├─ account.rs
│  │     ├─ download.rs
│  │     ├─ install.rs
│  │     ├─ ipc.rs
│  │     ├─ reboot.rs
│  │     ├─ service.rs
│  │     ├─ tray.rs
│  │     └─ wsl.rs
│  ├─ runtime/
│  │  ├─ machine.rs
│  │  ├─ mesh.rs
│  │  ├─ headscale.rs
│  │  ├─ tailscale.rs
│  │  ├─ docktail.rs
│  │  ├─ proxmox.rs
│  │  ├─ opentofu.rs
│  │  ├─ lxc.rs
│  │  └─ workload.rs
│  ├─ config.rs
│  ├─ state.rs
│  ├─ journal.rs
│  ├─ process.rs
│  ├─ secrets.rs
│  └─ error.rs
├─ runtime/
│  ├─ docktail.container
│  ├─ proxmox.container
│  └─ gnx-opentofu.service
├─ infra/
│  └─ opentofu/
│     ├─ versions.tf
│     ├─ variables.tf
│     ├─ main.tf
│     └─ outputs.tf
├─ guest/
│  ├─ bootstrap.sh
│  └─ units/
├─ workloads/
│  └─ <slug>/
│     ├─ workload.toml
│     ├─ <slug>.container
│     └─ health.json
├─ packaging/
│  └─ appimage/
├─ scripts/
│  ├─ build-windows.ps1
│  └─ build-appimage.sh
└─ tests/
   ├─ fixtures/
   ├─ integration/
   └─ acceptance/
```

No se crean archivos vacíos para cumplir el diagrama.

---

## 49. Reglas de código

### Rust

- módulos `snake_case`;
- tipos `PascalCase`;
- funciones con verbos;
- errores con códigos estables;
- unsafe aislado y documentado;
- no globals mutables;
- no traits para una sola implementación;
- no macros para ocultar flujo de negocio.

Preferir:

```text
HeadscaleController
MeshEnrollment
PodmanMachine
WindowsService
LxcBootstrap
WorkloadPolicy
```

Evitar:

```text
ProviderFactory
RuntimeManager
GenericEngine
PlatformHandler
```

---

## 50. Regla de abstracción

Una abstracción entra sólo cuando:

1. existen al menos dos consumidores reales;
2. comparten semántica;
3. reduce código o riesgo;
4. puede probarse;
5. no oculta autoridad.

No se crea:

```rust
trait MeshProvider
```

porque el producto soporta Headscale.

Sí puede existir:

```rust
struct HeadscaleController
```

---

## 51. Regla de dependencias

Cada crate externa documenta:

- problema;
- alternativa std;
- mantenimiento;
- licencia;
- tamaño;
- plataforma;
- CVEs;
- frecuencia de actualización.

Dependencias de runtime externas documentan además:

- versión;
- digest;
- fuente;
- compatibilidad;
- rollback;
- SBOM cuando esté disponible.

---

## 52. Estrategia de implementación

### Fase 0 — factibilidad

- crear repositorio/línea 1.x;
- fijar Headscale estable;
- fijar Tailscale client;
- fijar Docktail;
- ejecutar MESH-SVC-01;
- decidir go/no-go.

### Fase 1 — vertical slice Linux

- AppImage;
- Podman Machine;
- tailscaled;
- controller URL;
- Docktail;
- un workload;
- private health.

### Fase 2 — Proxmox y un LXC

- Proxmox Quadlet;
- OpenTofu;
- un LXC;
- Podman;
- Docktail por celda;
- workload.

### Fase 3 — Windows

- install;
- identidad;
- service;
- reboot;
- IPC;
- tray;
- init.

### Fase 4 — maintenance

- update;
- rollback;
- repair;
- uninstall;
- backups.

### Fase 5 — workloads oficiales

Se incorpora uno por vez.

No se inicia la fase siguiente sin evidencia de la anterior.

---

## 53. Política frente al legacy

```text
REFERENCE
REIMPLEMENT
IGNORE
```

### REFERENCE

Algoritmos, pruebas o conocimiento que ayuden a comprender el problema.

### REIMPLEMENT

Una responsabilidad necesaria reescrita bajo el contrato 1.x.

### IGNORE

Todo lo ligado sólo a:

- compatibilidad 0.x;
- schemas históricos;
- packaging anterior;
- payload generations;
- abstracciones no requeridas;
- workarounds no reproducibles.

No se copia un archivo legacy y luego se “limpia”.

---

## 54. Riesgos aceptados y no aceptados

### Aceptados temporalmente

- Headscale y Docktail evolucionan;
- Linux requiere matriz limitada;
- la primera versión soporta pocos workloads;
- algunos builds de desarrollo son unsigned.

### No aceptados

- declarar compatible algo no probado;
- usar Tailscale SaaS en silencio;
- almacenar pre-auth keys en config;
- usar API key Headscale global en release;
- exponer un socket Podman cruzando celdas;
- mantener fork privado sin decisión;
- ejecutar imágenes mutable;
- destruir datos en uninstall;
- omitir update y repair;
- reportar READY degradado.

---

## 55. Decisiones que negocio debe aprobar

1. Headscale es externo y obligatorio.
2. GNX no despliega ni respalda Headscale.
3. No existe fallback Tailscale SaaS.
4. La línea 1.x no migra 0.x.
5. Docktail se ejecuta por celda.
6. El release se bloquea mientras Headscale no soporte el contrato Services requerido.
7. No se acepta API key administrativa global como solución permanente.
8. Linux tendrá matriz limitada.
9. Update, repair y uninstall son parte del MVP.
10. Los releases distribuibles conservan firma e integridad.

---

## 56. Definition of Done

Quetzalcoatl Next está terminado cuando:

### Producto

- existe una línea greenfield 1.x;
- un crate Rust principal;
- una CLI;
- una configuración;
- un state nuevo;
- un runtime lógico;
- cero lectura de state 0.x;
- cero Docker Compose;
- cero Docker Engine;
- cero providers abstractos;
- cero fallback SaaS.

### Windows

- build x86_64;
- instalación nativa;
- identidad dedicada;
- Windows Service nativo;
- reboot/resume;
- Podman Machine propiedad del servicio;
- IPC;
- tray;
- update/rollback;
- repair;
- uninstall no destructivo.

### Linux

- AppImage x86_64;
- matriz publicada;
- FUSE/fallback probado;
- Podman Machine QEMU;
- KVM/nested virtualization;
- mismo runtime.

### Mesh

- controller URL obligatorio;
- Headscale estable fijado;
- enrolamiento tagged;
- pre-auth key eliminada;
- ningún contacto SaaS;
- peer directo o relay reportado;
- policy deny-by-default;
- recovery documentado.

### Docktail

- una instancia por celda;
- socket Podman local;
- socket Tailscale local;
- Services compatible con Headscale;
- credencial de mínimo privilegio;
- labels cerradas;
- drain/restart/rotation probados.

### Runtime

- tailscaled system service;
- Docktail Quadlet;
- Proxmox Quadlet;
- OpenTofu one-shot;
- LXC Podman + Quadlets;
- workloads healthy;
- private mesh endpoint healthy.

### Seguridad

- digest-pinned;
- downloads verificadas;
- no secrets en argv/log/state;
- no arbitrary execution;
- TLS;
- locks;
- rollback;
- threat tests.

### Evidencia

- unit;
- integration;
- Windows physical;
- Linux physical;
- controller outage;
- restart;
- update;
- repair;
- uninstall;
- MESH-SVC-01.

---

## 57. Goal final resumido

```text
                         Quetzalcoatl Next
                                  │
                   ┌──────────────┴──────────────┐
                   │                             │
                Windows                        Linux
                  EXE                         AppImage
                   │                             │
                   └─────── host preparation ────┘
                                  │
                       Podman Machine quetzalcoatl
                                  │
                               systemd
                 ┌────────────────┼───────────────────┐
                 │                │                   │
             tailscaled       Docktail            Proxmox
                 │                │                   │
                 └─────── Headscale externo           │
                                                     │
                                                  OpenTofu
                                                     │
                                                    LXC
                                                     │
                                                  systemd
                 ┌───────────────────────────────────┼──────────┐
                 │                                   │          │
             tailscaled                          Docktail     Podman
                 │                                   │          │
                 └──────── Headscale externo ────────┘      Quadlets
                                                                │
                                                            workloads
```

Propiedades:

- greenfield;
- lean;
- Rust-first;
- soberano;
- Headscale-first;
- sin Tailscale SaaS;
- Docktail por celda;
- una experiencia;
- un runtime;
- dos targets;
- actualización y recuperación;
- seguridad explícita;
- ninguna compatibilidad 0.x.

---

## 58. Regla de decisión futura

Cuando exista duda:

```text
¿esta capa resuelve un fallo real del recorrido aprobado?
```

Si la respuesta es no, la capa no entra.

Cuando exista duda entre:

```text
ocultar complejidad con un framework
```

y:

```text
mostrar el flujo real con código pequeño
```

se elige código pequeño.

Cuando exista duda entre:

```text
continuar silenciosamente
```

y:

```text
fallar con un diagnóstico accionable
```

se falla explícitamente.

---

## 59. Referencias de compatibilidad

Fuentes consultadas para esta propuesta:

- Headscale, requisitos y modelo de despliegue:
  https://github.com/juanfont/headscale/blob/main/docs/setup/requirements.md
- Headscale, registro de clientes y `--login-server`:
  https://github.com/juanfont/headscale/blob/main/docs/ref/registration.md
- Headscale, matriz de features:
  https://github.com/juanfont/headscale/blob/main/docs/about/features.md
- Headscale, feature gap de Tailscale Services:
  https://github.com/juanfont/headscale/issues/2845
- Docktail, sockets, credenciales y topologías:
  https://docktail.org/docs/
- Podman Machine:
  https://docs.podman.io/en/stable/markdown/podman-machine.1.html
- AppImage y FUSE:
  https://docs.appimage.org/user-guide/troubleshooting/fuse.html

Las versiones específicas deben fijarse al iniciar Fase 0. No se debe implementar
contra ramas `main`, tags `latest` ni documentación de una versión diferente
a la desplegada.

---

**Esta propuesta reemplaza únicamente el criterio arquitectónico de la nueva línea
Quetzalcoatl Next 1.x. No modifica, migra ni declara obsoleta por sí sola la línea
legacy. Su aprobación autoriza diseño greenfield; la implementación continúa sujeta
a los release gates, en especial MESH-SVC-01.**
