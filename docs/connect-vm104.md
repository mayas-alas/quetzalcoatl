# `gnx connect`: VM 104

Perfil de laboratorio de las fuentes `0.0.41`: `vm104` identifica
`gnx-incus-kini-lab`, con sesión como `gnxlab`. `list` muestra el perfil y el
estado del pin local; no descubre guests, consulta PVE ni indica que el túnel
esté disponible. La candidata `0.0.40` todavía muestra `debian`; no uses esa
cuenta para `no-auth-ssh` si conserva privilegios administrativos.

## Preparar la identidad SSH

El operador prepara `gnxlab` **dentro de la VM 104**, con contraseña bloqueada,
grupo privado propio, sin `sudo` ni acceso a sockets de motores privilegiados.
La preparación de la cuenta no habilita un listener ni cambia `debian`.
Desde una consola confiable y una cuenta administrativa autorizada de esa VM,
entra con `sudo -iu gnxlab` y verifica `hostname` e `id -un`. El SSH de
Tailcat se ejecuta como el usuario del listener; el nombre que muestra GnX
no cambia la cuenta efectiva en la VM.

Como `gnxlab`, prepara una vez la clave de host del SSH de Tailcat v0.7.0. Si ya
existe, se conserva. Necesita OpenSSL y las herramientas base de Debian.

```sh
(
  set -eu
  test "$(hostname)" = gnx-incus-kini-lab
  test "$(id -un)" = gnxlab
  umask 077
  dir="${XDG_CONFIG_HOME:-$HOME/.config}/tailcat/ssh"
  key="$dir/ssh_host_ed25519_key"
  mkdir -p "$dir"
  test ! -L "$key"
  if [ ! -f "$key" ]; then
    (set -C; openssl genpkey -algorithm ED25519 >"$key")
  fi
  # Ed25519 SubjectPublicKeyInfo: 12 bytes de cabecera + 32 de clave.
  public=$(openssl pkey -in "$key" -pubout -outform DER | base64 -w0)
  case "$public" in
    MCowBQYDK2VwAyEA*) ;;
    *) exit 1 ;;
  esac
  test "${#public}" -eq 60
  printf 'ssh-ed25519 '
  {
    printf '\000\000\000\013ssh-ed25519\000\000\000\040'
    printf '%s' "$public" | base64 -d | tail -c 32
  } | base64 -w0
  printf '\n'
)
```

Guarda **solo la línea pública** `ssh-ed25519 …` del canal de confianza en
`%LOCALAPPDATA%\GnX Core\connect\vm104.host.pub` (crea el directorio si falta;
usa texto ASCII o UTF-8 sin BOM, sin comentario). No uses la huella de
`/etc/ssh`: Tailcat tiene su propia identidad de SSH. Si la identidad cambia,
investiga en la consola antes de reemplazar el pin; `gnx connect` no acepta el
cambio automáticamente.

## Abrir una ventana de acceso

En la VM y como `gnxlab`, ejecuta esto en una consola privada (sin captura):

```sh
(
  set -eu
  test "$(hostname)" = gnx-incus-kini-lab
  test "$(id -un)" = gnxlab
  umask 077
  dir=$(mktemp -d)
  trap 'rm -f -- "$dir/invitation"; rmdir -- "$dir"' EXIT
  printf 'Archivo privado de invitación: %s/invitation\n' "$dir"
  TAILCAT_ADDR_FILE="$dir/invitation" \
    timeout --kill-after=5s 15m /usr/local/bin/tailcat --key=new serve no-auth-ssh \
    >/dev/null 2>&1
)
```

Transfiere la invitación desde ese archivo mediante el almacén protegido
acordado por el operador. No la consultes con `qm guest exec` (su resultado es
capturable), ni la pegues en chat, DNS, argumentos o historial. Quien obtenga
la invitación tiene acceso con los permisos de `gnxlab`, incluidas las
funciones que ofrezca su SSH integrado. No hay `authorized_keys` de cliente.
La VM necesita conectividad saliente para el rendezvous DERP; no se abre un
puerto entrante ni se reconfigura OpenSSH, el firewall o Proxmox.

## Conectar desde Windows

En PowerShell **sin elevar**, con el cliente OpenSSH de Windows:

```powershell
$gnx = "$env:ProgramFiles\GnX Core\gnx.exe"
& $gnx connect list
& $gnx connect vm104
```

Pega la invitación **solo en el prompt sin eco**. El CLI la entrega en memoria a
su adaptador Tailcat, por un pipe local del mismo usuario; OpenSSH verifica el
pin y controla la terminal. `exit` cierra la sesión de Windows. Para invalidar
la invitación antes del límite, detén el listener en la VM; no reuses un
listener con una invitación compartida por error.

Una compilación y `list` no acreditan acceso remoto. El cierre operativo exige
confirmar `hostname` e `id -un` en una sesión nueva y verificar que OpenSSH
rechaza una identidad distinta y que el listener detenido no admite otra
conexión.
