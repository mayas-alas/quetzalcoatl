# Quetzalcoatl GNX: entrega experimental

Esta entrega implementa dos binarios Rust: `quetzalcoatl-gnx-setup.exe` y `quetzalcoatl-gnx.exe`. Son los nombres fijos aprobados para Quetzalcoatl GNX. El segundo contiene la CLI y el punto de entrada del servicio Windows. Ambos deben permanecer juntos para instalar. La entrega local actual está en `dist/quetzalcoatl-gnx/`.

**No es una entrega de producción ni demuestra todavía el aislamiento completo.** Las pruebas automáticas comprueban el contrato de comandos y entradas; la creación de cuentas, el arranque de WSL desde el servicio y el reinicio completo necesitan una máquina de prueba. El instalador no se ejecutó contra este host.

## Uso

```text
quetzalcoatl-gnx-setup.exe preflight
quetzalcoatl-gnx.exe check
quetzalcoatl-gnx.exe status
```

`preflight` informa identidad, elevación y presencia del lanzador WSL sin modificar el equipo. Su presencia no demuestra que WSL2 funcione. La CLI devuelve un error explícito si el servicio no está instalado; no simula un estado saludable.

La instalación experimental requiere consola elevada, un archivo rootfs Ubuntu 24.04 obtenido de una fuente confiable, su SHA256 verificado por un canal confiable y el SID de la cuenta Windows que podrá consultar:

```text
quetzalcoatl-gnx-setup.exe install ROOTFS.tar SHA256 SID-DEL-USUARIO
```

El SID actual aparece en `preflight`. No introducir una contraseña en la línea de comandos. La cuenta dedicada usa una credencial aleatoria entregada al administrador de servicios de Windows.

## Comportamiento implementado

- Rechaza entradas inválidas y discrepancias de SHA256 antes de crear la cuenta.
- Prepara WSL sin registrar una distribución para la cuenta cotidiana.
- Crea una cuenta Windows estándar dedicada y le concede inicio como servicio; deniega inicio interactivo y por escritorio remoto.
- Protege datos privados con ACL y registra un servicio automático bajo esa cuenta.
- El servicio importa el rootfs en su propio contexto y prepara systemd y Podman rootless.
- La CLI usa un named pipe local, comprueba el PID del servicio y solo admite `check` y `status`. El servidor autentica el SID del cliente.
- Las consultas leen el estado del supervisor; no ejecutan comandos proporcionados por el usuario.

## Límites pendientes de resolver

- Se requiere Windows x64 instalado en `C:\Windows`; las rutas de esta primera implementación son fijas.
- Falta comprobar el perfil de la cuenta de servicio y el registro de WSL después de reiniciar sin iniciar sesión.
- No hay actualización, desinstalación ni recuperación automática de una instalación parcial. Ante colisiones el instalador se detiene y conserva datos para inspección.
- No se instala un Quadlet de aplicación: todavía no se definieron imagen, puertos ni volúmenes. Se prepara la capacidad de ejecutar Quadlets.
- El supervisor mantiene el entorno activo; `check` y `status` consultan la misma observación en esta versión.
- El paquete requiere dos EXE; aún no es un único instalador autocontenido ni está firmado.
- Una cuenta administradora elevada puede modificar la instalación. No se bloquea el uso de otras distribuciones por política global.

## Ciclo de verificación

Ejecutar `cargo test --locked`, construir con `cargo build --release --locked`, probar ayuda, preflight, ausencia de servicio y rechazo de comandos adicionales. Corregir fallos y repetir. La verificación completa requiere además las pruebas de identidad y reinicio descritas en la [arquitectura](arquitectura-wsl.md).

Los binarios y archivos temporales se excluyen de Git. El código fuente, lockfile y resultados documentados sí se publican.

La primera entrega se compiló y comprobó sin instalación privilegiada. La revisión actual y sus resultados están en la [auditoría de naming y seguridad](naming-auditoria.md). No se considera cumplido el objetivo completo de instalación y aislamiento.
