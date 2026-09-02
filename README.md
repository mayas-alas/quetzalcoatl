# Quetzalcoatl

Base arquitectónica para instalar el mismo runtime de servicios en dos hosts:

- Windows 11 x86_64: Podman 6+ usa una Podman Machine WSL llamada
  `quetzalcoatl`, propiedad de una cuenta local dedicada.
- Linux x86_64: Podman 6+ corre directamente sobre el host, después de validar
  systemd, cgroup v2 y KVM.

En ambos casos systemd gobierna los mismos Quadlets. El alcance actual es
arquitectura y auditoría; todavía no hay implementación.

## Estado

Cada mesh tendrá un solo control plane y un `control_server` estable. La primera
instalación usa `create`; Windows y Linux posteriores usan `join`, conservan una
identidad propia y no arrancan otro Headscale. `gnx-netd` será un fork mínimo de
un daemon BSD-3 maduro. Docktail sigue condicionado por su brecha de Services y
el producto no debe reportar `READY` mientras sea requisito.

## Documentos

- [Arquitectura](docs/architecture.md): modelo objetivo, flujos y árbol futuro.
- [Auditoría](docs/audit.md): hechos comprobados, bloqueos y preguntas abiertas.
- [ADR-0001](docs/decisions/0001-network-daemon.md): decisión sobre `gnx-netd`.
- [ADR-0002](docs/decisions/0002-mesh-identity-and-endpoint.md): identidad,
  multiinstalación, endpoint y custodia de credenciales.
- [Gateway de agentes](docs/agent-gateway.md): acceso local para CLIs, fuera del
  runtime distribuido.

## Principios de esta base

1. Dos adaptadores de host; un solo contrato de runtime.
2. En Windows, Podman Machine es la distribución WSL; WSL no vive dentro de
   Podman Machine.
3. En Linux no se agrega otra VM: Podman y los Quadlets son nativos.
4. `gnx-netd`, no Docktail, se registra mediante
   `gnx connect --control-server=https://mesh.gnx`.
5. Imágenes y artefactos se fijarán por versión, digest y checksum en cada
   release; no se usará `latest`.
6. Ningún gate fallido se presenta como instalación lista.
7. Cada FQDN tiene un solo escritor; una credencial DDNS maestra nunca llega a
   las instalaciones miembro.
