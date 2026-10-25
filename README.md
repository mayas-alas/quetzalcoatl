# GnX Core

Implementación nueva y parcial; **todavía no es un producto instalable**.
El legado queda intacto. Alcance y decisiones: [scope.md](scope.md) y
[architecture.md](architecture.md).

## Lo entregado

- Un ejecutable Rust: `gnx start`, `gnx stop`, `gnx status`; entrada SCM interna.
- Pipe local autenticado contra PID del servicio y token SYSTEM; ACL admin/SYSTEM,
  frames limitados, tiempos acotados y operaciones fijas, sin shell libre.
- Frontera a WSL con token/perfil/entorno de la cuenta dedicada, comprobación de
  SID y GUID de registro WSL. No usa la WSL del operador.
- Start/stop delegados a un target systemd; intención mínima persistente para
  recuperación. No hay supervisor ni lifecycle Rust por contenedor.
- Configuración externa de target, propagación `PartOf` y WSL. Los cuatro
  `.container` completos y sus inputs offline **aún no están entregados**.

Identidad nueva: `GnXCore`, `\\.\pipe\GnX.Core.v1`, `gnxcoresvc`, `gnx-core`,
`%ProgramData%\GnX Core`. No adoptar recursos por coincidir el nombre.

## Compilar

```powershell
cargo build --release --locked
./target/release/gnx.exe --help
./target/release/gnx.exe --version
```

No se instala un servicio al compilar ni al consultar status. Sin una instalación
nueva preparada, los comandos informan el bloqueo; nunca usan el legacy como fallback.
`status` solo enumera WSL en ejecución: no selecciona una distro para ejecutar
systemctl, porque eso podría despertarla. No acredita salud de HTTPS/mesh/PVE.

## Retomar sin ampliar el esfuerzo

1. Leer scope/arquitectura vigentes y `git status`/diff; conservar trabajo ajeno.
2. Mantener Rust + configuración externa + scripts pequeños. Lifecycle en
   systemd/Quadlets/PVE. Sin templates incrustados, frontend framework ni restaurar legacy.
3. **No crear código de tests, fixtures, mocks, harnesses ni infraestructura de
   pruebas.** El usuario difirió ese paradigma fuera de este esfuerzo. Solo
   compilación/formato y comprobaciones externas mínimas/sintéticas; sin instalar
   un entorno de prueba. No confundirlas con aceptación real.
4. Hacer commits pequeños con `Hector <hector@email.gnx>`; no hacer push sin permiso.
5. Siguiente hito: preparar inputs offline verificables y resolver solo las Dxx
   dependientes del instalador, sin tocar la instalación antigua.

### Contrato de preparación pendiente del instalador

El runtime **no crea** cuenta, perfil, distro ni credenciales por recibir start/status.
El instalador deberá crear una identidad no administradora y registrar su SID y
GUID WSL reales en `identity.json` (`version`, `sid`, `distribution_id`). El mismo
directorio contendrá `account.dpapi`, contraseña UTF-8 protegida con DPAPI de
máquina. Directorio/archivos solo SYSTEM y Administrators, sin reparse points;
ACL aplicada antes de escribir. No fabricar este registro para adoptar un recurso.
El servicio deberá instalarse bajo LocalSystem con argumento `--service` y política
de recuperación SCM. Estos son contratos, **no un instalador ya implementado**.

Pendiente: MSI/EXE autocontenido, preparación de identidad/WSL, rootfs/OCI fijados,
Quadlets completos, CA/acceso HTTPS local, enrolamiento protegido, entrada app/CLI
al LXC y OIDC RustFS/NetBird con Pocket ID. No hay URLs ni logins aceptados aún.
