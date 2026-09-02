# Quetzalcoatl

Base arquitectónica para instalar el mismo runtime de servicios en dos hosts:

- Windows 11 x86_64: Podman 6+ usa una Podman Machine WSL llamada
  `quetzalcoatl`, propiedad de una cuenta local dedicada.
- Linux x86_64: Podman 6+ corre directamente sobre el host, después de validar
  systemd, cgroup v2 y KVM.

En ambos casos systemd gobierna los mismos Quadlets. El alcance actual es
arquitectura y auditoría; todavía no hay implementación.

## Estado

La topología Headscale + `gnx-netd` + Dockur/Proxmox es viable con gates físicos
y de red. `gnx-netd` será un fork mínimo de un daemon BSD-3 maduro; la
reimplementación Rust queda sólo como investigación. Docktail no puede
considerarse integrado: depende de una API de Services que sigue abierta como
brecha de compatibilidad en Headscale. El producto no debe reportar `READY`
mientras Docktail sea requisito.

## Documentos

- [Arquitectura](docs/architecture.md): modelo objetivo, flujos y árbol futuro.
- [Auditoría](docs/audit.md): hechos comprobados, bloqueos y preguntas abiertas.
- [ADR-0001](docs/decisions/0001-network-daemon.md): decisión sobre `gnx-netd`.

## Principios de esta base

1. Dos adaptadores de host; un solo contrato de runtime.
2. En Windows, Podman Machine es la distribución WSL; WSL no vive dentro de
   Podman Machine.
3. En Linux no se agrega otra VM: Podman y los Quadlets son nativos.
4. `gnx-netd`, no Docktail, se registra mediante
   `gnx connect --control-server=<HEADSCALE_URL>`.
5. Imágenes y artefactos se fijarán por versión, digest y checksum en cada
   release; no se usará `latest`.
6. Ningún gate fallido se presenta como instalación lista.
