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
gnx logs
gnx init
gnx repair
gnx update --from <artefacto> --sha256 <sha256>
gnx uninstall
```

La primera convergencia configura el bootstrap DNS del Headscale propio y entrega
su pre-auth key exclusivamente por entrada estándar:

```powershell
$ControllerAddress = Read-Host "IP real de Headscale"
Get-Content -Raw C:\ruta-segura\headscale-preauth.key |
  gnx init --controller-address $ControllerAddress --mesh-auth-stdin
```

GNX mantiene `controlplane.node.gnx` como nombre HTTPS y propaga su resolución a
Windows, Podman Machine y LXC. Docktail usa el socket del tailscaled local, que es
el cliente inscrito mediante ese endpoint Headscale.

El runtime usa Podman Machine `quetzalcoatl`, systemd, tailscaled, Docktail y
Dockur Proxmox. OpenTofu corre dentro del LXC dedicado `gnx-infra-runner` y desde
allí converge los LXC de workloads con el provider BPG/Proxmox.

En Windows, el servicio corre bajo la cuenta local aislada `gnx-runtime`; WSL y
Podman Machine pertenecen a ese perfil y no al usuario interactivo. El tray se
inicia al terminar la instalación y vuelve al iniciar sesión. La trazabilidad
persistente se consulta con `gnx logs` o en
`C:\ProgramData\QuetzalcoatlNext\logs\gnx.jsonl`.

Los endpoints de referencia son `https://headscale.node.gnx` y
`https://controlplane.node.gnx`. GNX conserva el controller configurado y valida
su contrato HTTPS/DNS/TLS sin políticas por marca.

Documentación vigente:

- [Tracker](IMPLEMENTATION-TRACKER.md)
- [Arquitectura](docs/architecture.md)
- [Build por host](docs/build.md)

Los builds quedan en `dist/`; las dependencias externas fijadas están en
`dependencies.lock.toml`. Backup y recovery no forman parte del MVP.
