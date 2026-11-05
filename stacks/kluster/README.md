# GnX Kluster

Perfil fijo seleccionado por `gnx deploy --stack kluster`. Es distinto de
`gnx deploy` (private-services) y de la admision generica de carpetas.

El servicio verifica la identidad de su cuenta/WSL dedicada y persiste la entrada
exacta. Prepara kernel.panic=10 y vm.overcommit_memory=1 en esa WSL, conserva los
valores anteriores en /var/lib/gnx-core/kluster-kernel/original.conf y registra una
unidad systemd anterior a compute. Los parametros pertenecen al kernel compartido
por PVE y sus LXC. No modifica .wslconfig, memoria de WSL ni una WSL personal.

El perfil crea un LXC privilegiado propio (2 CPU, 3 GiB RAM, 512 MiB swap, 20 GiB
disco), fijando descripcion, recibo, base verificada y /dev/kmsg con escritura
denegada. No convierte ni adopta 1001/1002, no modifica la admision generica.
Preserva las reservas incompletas; la ausencia del guest o una entrada diferente
requieren revision explicita y no asignan otro VMID silenciosamente.

Dentro del guest instala K3s, un registry Podman/Quadlet en loopback con TLS y CA
local, una aplicacion HTTP y un emisor de trafico en gnx-demo, y el chart del fork
mayas-alas/gnx-kubeshark en gnx-observe. Las descargas tienen SHA256 y las imagenes
del registry, demo y Kubeshark estan fijadas por digest. Las imagenes internas
de K3s corresponden a su distribucion fijada. La demo se publica y consume por digest en el
registry local. No se compila en el LXC ni se desactiva la verificacion TLS.

Las versiones y procedencia estan en assets.json. El chart se descarga del fork
fijado; las herramientas kftui/kftray y kubeshark utilizan binarios publicados por
sus proyectos upstream, con la version correspondiente al fork y SHA256 fijados.
No se presentan como binarios compilados por los forks de mayas-alas.

El archivo AppImage del tray queda en /var/lib/gnx-app/tools/kftray.AppImage para
instalarlo en un escritorio compatible; el LXC no inicia una interfaz grafica.
La integracion visual con el escritorio Agent requiere verificar sus dependencias
y credenciales por separado. No se copia el kubeconfig administrador a Agent.

## Acceso

Las UIs son ClusterIP, sin ingress publico. La autenticacion de acceso la aporta
Kubernetes al autorizar port-forward; Kubeshark esta limitado a observar gnx-demo.
Desde una consola administrativa del guest:

```bash
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
/usr/local/bin/k3s kubectl -n gnx-demo port-forward svc/demo 8080:8080
# En otra consola:
/usr/local/bin/k3s kubectl -n gnx-observe port-forward svc/kubeshark-front 8899:80
```

El kubeconfig permanece root:0600 y no se imprime. Un cliente externo debe recibir
un kubeconfig protegido, CA valida y permisos acordes a su uso. La IP DHCP debe
mantenerse alcanzable; no se presupone publicacion por los dominios GnX.

## Prueba en el laboratorio 1002

Con el codigo de esta rama disponible en PVE, ejecutar:

```bash
bash stacks/kluster/scripts/validate-lab.run
```

Este recorrido modifica exclusivamente el guest de laboratorio 1002 y los dos
sysctls compartidos autorizados; sube su limite de RAM a 3 GiB tras comprobar RAM
disponible en PVE. Usa los mismos scripts de kernel e instalacion que el servicio.
No registra el guest manual como propiedad del producto y no ejecuta el pipe de
la nueva CLI. Conserva cluster, datos, CA y configuracion base del K3s ya instalado.

La aceptacion exige nodo Ready, registry TLS disponible, demo respondiendo y
workloads de Kubeshark listos. La captura real y el tray requieren comprobacion
visual posterior. Ningun check estatico acredita esas condiciones en runtime.

El EXE y su publicacion por Actions se construyen despues de validar el stack.
La fuente prepara el comando de la siguiente entrega; el CLI 0.0.38 instalado
no admite esta seleccion todavia. La prueba manual valida scripts y runtime,
no la actualizacion MSI/EXE ni el recorrido del servicio Windows de esa entrega.
