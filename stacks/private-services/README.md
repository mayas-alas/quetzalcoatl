# Stack privado

RustFS, Pocket ID y NetBird con su dashboard se ejecutan en un LXC separado del
núcleo. `quadlets/` conserva las definiciones sin secretos; no es un catálogo ni
una copia del código de las aplicaciones.

## Red y persistencia

- `private.network` crea una red Podman bridge con DNS interno.
- Los cuatro contenedores usan esa red; ninguno usa `Network=host`.
- Los datos se mantienen en directorios de `/var/lib/gnx-app/`, mediante bind mounts.
- El caso privado usa `172.30.0.100/24` en el bridge PVE y solo publica sus puertos
  de aplicación sobre esa IP. La preparación configura PVE y NetworkManager del guest.
- El gateway del núcleo termina HTTPS. La clave privada de la CA permanece en el
  núcleo; el guest recibe el certificado raíz público para verificar OIDC.
- Los secretos propios de las aplicaciones sí residen en el guest, bajo
  `/etc/gnx-app/config/`, protegidos para root. No se publican en el navegador.

## Preparar la entrada

La carpeta final de `gnx deploy` debe contener los cinco Quadlets de `quadlets/`
y estos archivos bajo `config/`, protegidos y fuera de Git:

| Archivo | Contenido |
| --- | --- |
| `private-stack.json` | Marcador versionado que solicita la publicación de este stack. |
| `root.crt` | Certificado público exacto de la CA del nodo; no una clave privada. |
| `pocket.env` | ENCRYPTION_KEY persistente y STATIC_API_KEY administrativa dedicada. |
| `rustfs.env` | Credenciales iniciales RustFS. La preparación añade su cliente OIDC sin reemplazarlas. |
| `netbird.yaml` | Configuración del servidor combinado: issuer `https://netbird.gnx/oauth2`, backend `:8081`, STUN 3478, claves persistentes y callbacks del dashboard. |

Usar valores aleatorios propios y conservarlos. No copiar los secretos ni las
identidades de otra instalación. Las imágenes están fijadas por digest y requieren
conectividad a sus registros; no se construyen desde Containerfile.

## Configuración de identidad

La preparación fija `scripts/enroll-stack.run` se ejecuta dentro del guest. Crea
clientes dedicados de Pocket ID y añade ese proveedor en NetBird. Configura RustFS
con el cliente separado y la política S3 `readwrite`, no administración global.
Los usuarios de Pocket ID los administra el operador; no se abre el registro público.

Solo una instancia NetBird sin propietario puede recibir el administrador local
de recuperación inicial. Su credencial y el estado de la integración quedan en
`/etc/gnx-app/config/enrollment.json`, root 0600. Una instancia ya configurada sin
ese recibo no se adopta ni se restablece. Los tokens de preparación no se publican.

## Acceso

| URL | Uso |
| --- | --- |
| `https://pocket.gnx/setup` | Alta inicial del operador y su passkey real. |
| `https://rustfs.gnx/rustfs/console/` | Consola RustFS con Pocket ID. |
| `https://s3.gnx` | API S3, con credenciales S3 o STS apropiadas. |
| `https://netbird.gnx` | Dashboard, API, OAuth2, gRPC y relay mediante el gateway. |

La passkey debe pertenecer al operador: no se generan autenticadores simulados
para afirmar que hubo login humano. La configuración de OIDC no convierte a una
cuenta Pocket ID en administradora global de RustFS.

### Primer acceso a Pocket ID

Configurar los clientes OIDC con `STATIC_API_KEY` no crea una cuenta humana ni
una passkey. La cuenta técnica de esa API no sirve para iniciar sesión desde el
navegador. Antes de usar los botones Pocket ID de RustFS o NetBird:

1. Abrir `https://pocket.gnx/setup` con HTTPS confiable.
2. Completar el alta inicial con los datos del operador.
3. Elegir **Add Passkey** y registrar un autenticador real compatible.
4. Volver a la aplicación e iniciar sesión con esa passkey.

Es el [flujo nativo de Pocket ID](https://pocket-id.org/docs/setup/installation#create-the-admin-account),
no otro mecanismo de autenticación de GnX. La página de setup solo está disponible
hasta crear la primera cuenta humana. No restablecer una instalación ya configurada
para repetir este procedimiento.

Un error al iniciar sesión sin cuentas/passkeys no demuestra un fallo del proxy
OIDC. Si falla **registrar** la passkey, distinguir el error del navegador o
autenticador de una respuesta del servidor; no omitir TLS, simular una passkey ni
añadir un login alternativo propio para sortearlo.

El acceso remoto mantiene los requisitos del nodo: DNS, ACL y confianza de cada
cliente. STUN necesita UDP directo; el proxy HTTPS y el portproxy TCP Windows no
lo sustituyen. No se declara conectividad completa entre peers por abrir el dashboard.
