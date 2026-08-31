# Quetzalcoatl Next (GNX)

MVP greenfield para Windows y Linux. No adopta ni migra estado 0.x.

La primera instalación comienza abriendo el artefacto, sin comandos previos:

- Windows: `gnx-windows-x86_64.exe`
- Linux: `gnx-x86_64.AppImage`

El instalador prepara WSL o QEMU, instala Podman CLI, agrega `gnx` al `PATH` y
registra el servicio de arranque. Después, una shell nueva dispone de:

```text
gnx
gnx status
gnx doctor
gnx init
gnx repair
gnx update --from <artefacto> --sha256 <sha256>
gnx uninstall
```

El runtime usa Podman Machine `quetzalcoatl`, systemd, tailscaled, Docktail y
Dockur Proxmox. OpenTofu corre dentro del LXC dedicado `gnx-infra-runner` y desde
allí converge los LXC de workloads con el provider BPG/Proxmox.

Los endpoints de referencia son `https://headscale.node.gnx` y
`https://controlplane.node.gnx`. GNX conserva el controller configurado y valida
su contrato HTTPS/DNS/TLS sin políticas por marca.

Documentación vigente:

- [Tracker](IMPLEMENTATION-TRACKER.md)
- [Arquitectura](docs/architecture.md)
- [Build por host](docs/build.md)

Los builds quedan en `dist/`; las dependencias externas fijadas están en
`dependencies.lock.toml`. Backup y recovery no forman parte del MVP.
