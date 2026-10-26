# GnX Core

Tu nodo privado en Windows, operado con una CLI pequeña y herramientas nativas.

GnX separa el runtime de tu entorno personal: una cuenta Windows y una WSL
dedicadas ejecutan **Tailscale, Pi-hole, Caddy y Proxmox VE** bajo Quadlets/systemd.
El servicio Windows mantiene la frontera de identidad y acceso; no es otro
supervisor de contenedores.

## Para qué usarlo

- Abrir **https://app.gnx** para conocer el avance local, sin esperar una key mesh.
- Acceder a Proxmox mediante **https://compute.gnx**, con TLS y autenticación nativos.
- Pausar y reanudar el nodo sin borrar datos ni detener tus otras WSL.
- Como destino del producto: entregar Quadlets/configuración desde app o CLI
  a un LXC, para RustFS, Pocket ID y NetBird. Este despliegue todavía no está
  disponible en la versión inicial.

## Instalar y operar

Windows x64 con WSL2 disponible. El MSI/EXE incluye filesystem e imágenes OCI;
la preparación no debe descargar dependencias. El enrolamiento mesh sí necesita
acceso a su plano de control.

Obtén la versión desde [Releases](https://github.com/mayas-alas/quetzalcoatl-next/releases).
La instalación nueva usa `GnXCore` / `gnxcoresvc` / `gnx-core`; no migra ni elimina
el GnX anterior. Un conflicto de puertos, nombres o propiedad se informa.

Abre PowerShell como administrador para operar el pipe de control:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx status
& $gnx start
& $gnx enroll
& $gnx password
& $gnx stop
```

Mientras exista el GnX anterior, usa esta ruta explícita: el instalador no
reescribe PATH ni hace que el comando antiguo apunte al producto nuevo.

`status` no despierta una distro detenida. `start` no instala ni actualiza.
`stop` delega en systemd y conserva datos e identidad; no usa `wsl --shutdown`.
No introduzcas keys ni contraseñas en argumentos, navegador o chat.

## HTTPS local y por Tailscale

MagicDNS/split DNS resuelve nombres; no instala certificados. El dominio `gnx`
debe usar el Pi-hole del nodo y las ACL permitir DNS TCP/UDP 53 y HTTPS TCP 443.

Ambas URLs usan la misma CA privada del nodo, también al acceder por la mesh.
El servicio importa automáticamente su CA en el almacén raíz de Windows mediante
`certutil`, igual que en v0.0.6: no se exige importación manual en el host para
clientes que usan ese almacén. Un navegador con almacén propio puede necesitar
confianza adicional. En Android/OPPO, si la CA no está ya confiada, importa la **CA pública**
como certificado de autoridad en los ajustes de seguridad; el nombre del menú
depende de Android/ColorOS y el navegador debe admitir esa confianza.

Obtén la CA en [app.gnx/gnx-labs-root.crt](https://app.gnx/gnx-labs-root.crt)
desde un cliente que ya valide HTTPS, y contrasta su huella SHA-256 por un canal
confiable antes de instalarla. No distribuyas claves privadas ni uses excepciones
de certificado como solución. La CA del GnX anterior no acredita la del nodo nuevo;
cambiar de nodo o de CA exige verificar y distribuir de nuevo la confianza.
El HTTPS automático de [Tailscale](https://tailscale.com/kb/1153/enabling-https)
emite certificados públicos para nombres del tailnet bajo `.ts.net`; no convierte
una CA privada para `.gnx` en una CA confiada por el dispositivo.

## Base versionada

La base procede de Fedora bootc, fijada por digest. Una imagen propia sellada
podrá servir como base común para WSL y LXC, con configuración y datos separados.
En LXC vivirán las aplicaciones, no otra copia de los cuatro servicios del nodo.
La exportación OCI→WSL no promete capacidades bootc de upgrade/rollback dentro
de WSL o LXC: cada capacidad se habilita solo cuando sea compatible y comprobada.

## Desarrollo

```powershell
cargo build --release --locked
./scripts/build.ps1 -Package
```

El empaquetado exige inputs locales completos en `build/payload`; no los sustituye
por descargas silenciosas. Diagnóstico: systemctl/journalctl y SCM/Visor de eventos.

[Cambios v0.0.7](releases/v0.0.7/CHANGELOG.md) · [Alcance](scope.md) · [Arquitectura](architecture.md) ·
[Reglas para agentes](.AGENTS)

## Licencia

Código propio de GnX: [GNU AGPLv3](LICENSE), `AGPL-3.0-only`.
Las imágenes y dependencias de terceros conservan sus respectivas licencias.
