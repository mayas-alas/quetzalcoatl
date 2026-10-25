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

```powershell
gnx status
gnx start
gnx stop
```

`status` no despierta una distro detenida. `start` no instala ni actualiza.
`stop` delega en systemd y conserva datos e identidad; no usa `wsl --shutdown`.
No introduzcas keys ni contraseñas en argumentos, navegador o chat.

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

[Cambios](CHANGELOG.md) · [Alcance](scope.md) · [Arquitectura](architecture.md) ·
[Reglas para agentes](.AGENTS)
