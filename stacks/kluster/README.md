# GnX Kluster

Perfil de infraestructura seleccionado con `gnx deploy --stack kluster`.
El servicio verifica su identidad Windows y WSL, prepara el kernel y despliega
un LXC Kubernetes propio. El instalador Windows incorpora el comando; el despliegue
se solicita explícitamente después de instalar.

## Componentes

| Ámbito | Componente | Responsabilidad |
| --- | --- | --- |
| LXC | `gnx-kluster-registry` | Registro OCI local con TLS y CA privada. |
| `gnx-services` | `service-health` | Endpoint HTTP para comprobar conectividad. |
| `gnx-services` | `http-probe` | Comprobaciones HTTP periódicas con resultado en los logs. |
| `gnx-observability` | `network-console` | Consola de observabilidad de red. |
| `gnx-observability` | `traffic-analysis` | Coordinación y análisis del tráfico capturado. |
| `gnx-observability` | `packet-capture` | Captura de paquetes y trazado TLS mediante eBPF. |

La release Helm se llama `gnx-observability`. Kubeshark es la dependencia de
observabilidad; las imágenes, etiquetas y endpoints internos que sus binarios
requieren conservan su identidad técnica. El servicio `network-console` es la
entrada del producto; el alias interno del proveedor se conserva por compatibilidad.
No se atribuye a GnX la autoría de esas dependencias.

## Despliegue controlado

El perfil prepara `kernel.panic=10` y `vm.overcommit_memory=1` en la WSL dedicada,
guarda los valores anteriores y registra una unidad systemd anterior a compute.
Los parámetros pertenecen al kernel compartido por PVE y sus LXC. No modifica
`.wslconfig` ni reinicia WSL.

Crea un LXC privilegiado propio con 2 CPU, 3 GiB de RAM, 512 MiB de swap y 20 GiB
de disco. Reserva el VMID, verifica recibos y descripción, y pasa `/dev/kmsg` con
escritura denegada. Antes del arranque restaura únicamente CAP_SETFCAP frente al
perfil Fedora, conserva las otras siete capacidades descartadas y guarda un
respaldo PVE. Una política distinta requiere revisión.

Las versiones, fuentes y hashes están en `assets.json`. El chart procede del fork
mayas-alas/gnx-kubeshark. K3s, Helm, kubeshark, kftui y kftray son binarios upstream
verificados; no se compilan dentro del LXC. Las imágenes se fijan por digest.
El endpoint HTTP se publica en el registro local y se consume por digest con TLS.

El trazador tiene límite de 1 GiB y reserva de 256 MiB. Los límites del coordinador,
capturador y consola son 512, 384 y 128 MiB. La suficiencia de estos límites se
comprueba en la aceptación manual; el límite previo del trazador provocó OOM.

`scripts/reconcile-observe.run` actualiza observabilidad mediante Helm sin
reiniciar K3s ni el registro. Verifica el chart almacenado, exige 1 GiB disponible
y bloquea operaciones pendientes. Conserva el valor operativo de
`DISSECTION_ENABLED` que mantiene el coordinador para evitar imponer otro valor
ni forzar su propiedad. La salud exige servicios disponibles, una petición HTTP
correcta y 30 segundos sin reinicios del capturador.

El AppImage del cliente gráfico queda en `/var/lib/gnx-app/tools/kftray.AppImage`.
Su instalación en un escritorio compatible y sus credenciales se verifican por
separado; el LXC no inicia una interfaz gráfica ni copia el kubeconfig a Agent.

## Acceso

Los servicios son ClusterIP. El acceso se autoriza mediante Kubernetes y
port-forward desde una consola administrativa del guest:

```bash
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
/usr/local/bin/k3s kubectl -n gnx-services port-forward svc/service-health 8080:8080
# En otra consola:
/usr/local/bin/k3s kubectl -n gnx-observability port-forward svc/network-console 8899:80
```

El kubeconfig permanece root:0600. Un cliente externo necesita credenciales
protegidas, CA válida y permisos acordes a su uso. La captura se limita a
`gnx-services`; no se presupone publicación por los dominios GnX.

## Aceptación y laboratorio

La candidata 0.0.39 incorpora estas fuentes. Compilar y empaquetar no acredita
instalación Windows, disponibilidad del stack ni captura real: se comprueban
manualmente con el instalador exacto descargado.

`validate-lab.run` se limita al laboratorio manual 1002 y no lo adopta como
propiedad del servicio. El esquema de recursos anterior requiere una migración
explícita: la reconciliación detecta esa instalación y se detiene antes de crear
otra en paralelo. No elimina namespaces, releases ni datos para cambiar nombres.
El despliegue del servicio crea su propio LXC; no transforma 1001 ni 1002.
