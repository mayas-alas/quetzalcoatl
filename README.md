# GNX

GNX compila dos ejecutables desde el mismo código: `gnx.exe` para Windows y
`gnx` para Linux/WSL. La interfaz tiene sólo tres capacidades:

```text
gnx access      # Tailscale, Services y Pi-hole para Split DNS .gnx
gnx compute     # ciclo de vida y salud del servicio de cómputo
gnx controller  # entrada HTTP y CA autónomo opcional para HTTPS .gnx
```

Windows es un puente delgado: valida la misma configuración y delega la acción
al binario Linux dentro de WSL. Tailscale entrega el transporte y el TLS
automático de `*.ts.net`; Pi-hole responde `.gnx` cuando el tailnet dirige esa
zona por Split DNS. El CA autónomo conserva una segunda ruta HTTPS para `.gnx`,
pero confiar en él siempre es una decisión explícita del operador.

## Inicio

1. Copiar `config/gnx.example.toml` a `gnx.toml` y sustituir el FQDN de ejemplo.
2. Instalar el bundle generado por `packaging/windows/build.ps1`.
3. Ejecutar, en orden: `gnx compute apply`, `gnx controller apply` y
   `gnx access configure`.
4. Aprobar `svc:compute` en Tailscale si el reporte lo solicita.
5. En DNS del tailnet, añadir el nameserver reportado y restringirlo a `gnx`.

Los detalles, diagramas y gates están condensados en
[arquitectura](docs/arquitectura.md) y [operación](docs/operar.md).

## Licencia

GNX usa `AGPL-3.0-only`; las dependencias conservan sus licencias y
atribuciones. La rama histórica `legacy` permanece separada y sin modificaciones.
