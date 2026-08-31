# Quetzalcoatl Next (GNX)

Base greenfield de Quetzalcoatl Next. No adopta, migra ni elimina estado 0.x.

La primera instalación se inicia abriendo el artefacto, sin comandos previos:

- Windows: `gnx-windows-x86_64.exe`
- Linux: `gnx-x86_64.AppImage`

El instalador solicita elevación, instala WSL cuando Windows lo necesita, instala
Podman CLI, agrega `gnx` al `PATH` y registra el servicio de arranque. Al terminar,
una shell nueva dispone de:

```text
gnx
gnx status
gnx doctor
gnx init
gnx repair
gnx update --from <artefacto> --sha256 <sha256>
gnx uninstall
```

Los controllers HTTPS de referencia son `https://headscale.node.gnx` y
`https://controlplane.node.gnx`. GNX conserva exactamente el endpoint configurado;
valida HTTPS/DNS/TLS sin aplicar políticas por marca.

Documentos normativos:

- `IMPLEMENTATION-TRACKER.md`
- `docs/architecture.md`
- `docs/build.md`

Los builds quedan en `dist/` y las dependencias externas fijadas en
`dependencies.lock.toml`.
