# OpenUI en el LXC de Kluster

Dos demos reales bajo Podman y Quadlets, en /stacks/openui. El LXC existente se conserva. Si runtime.json declara mode=existing se conserva
su OmniRoute; mode=new prepara una instancia nueva con la imagen publicada fijada
por digest, sin sustituir contenedores existentes. Este stack es independiente del instalador
Windows; no cambia el motor a Kubernetes ni amplía gnx deploy.

| Dirección | Uso |
| --- | --- |
| app.gnx | Portada HTML/CSS del núcleo |
| f1.gnx | F1 Dashboard con herramientas OpenF1 y streaming AG-UI |
| intelligent.gnx | Open Intelligent UI |
| omniroute.gnx | Consola de la instancia OmniRoute existente |
| api.gnx/v1 | API OpenAI-compatible, con API key de OmniRoute |

## Configuración

1. Copiar esta carpeta al LXC autorizado como /stacks/openui.
2. Inspeccionar la instancia OmniRoute existente y completar runtime.json con su
   nombre exacto, endpoint privado y la IP observada del guest. No seleccionar
   otro contenedor por coincidencia de nombre.
3. Ejecutar scripts/build.run: descarga las dos revisiones de sources.json y
   construye sus imágenes con lockfiles, Node por digest y Next.js 16.4.5.
4. Ejecutar scripts/install.run: verifica el modo y contenedor declarados, prepara
   directorios privados y arranca únicamente las dos demos. No reemplaza
   OmniRoute ni modifica sus llaves, modelos, volúmenes o configuración.
5. Configurar el proveedor/modelos en la consola nativa de OmniRoute y guardar
   su API key de cliente y el modelo elegido en secrets/apps.env (0600). El
   modelo predeterminado es `entry-stack`; se puede elegir otro al configurar.
   Reiniciar solo openui-f1.service y openui-intelligent.service.

Las páginas pueden abrirse antes de configurar inferencia: el chat devuelve 503
hasta tener endpoint, API key y modelo. El modelo F1 debe soportar function tools
y streaming; una respuesta de texto no acredita ese recorrido.

El instalador genera una contraseña privada de acceso a las demos en
secrets/demo-password y un hash bcrypt en secrets/ingress.env. La imagen existente
de OmniRoute se usa, sin red, solo para la operación bcryptjs; si esa dependencia
no está disponible, la instalación se detiene antes de publicar las demos.

## Publicación en el gateway del nodo

Copiar secrets/ingress.env al directorio protegido del núcleo y cargarlo mediante
EnvironmentFile en el Quadlet del gateway. gateway.caddy usa la IP observada en
GNX_OPENUI_GUEST y ese hash. Instalarlo en el directorio de imports ya existente,
validar Caddy antes de recargar y conservar su configuración anterior.

La v0.0.37 incluye los dominios en hosts, DNS y hoja TLS, conservando la CA.
La ruta al guest debe corresponder a su red PVE comprobada. No exponer puertos
del LXC a Internet. Publicar api.gnx únicamente después de verificar que OmniRoute
rechaza peticiones sin API key; Origin no autentica personas.

## Persistencia y procedencia

sources.json fija las revisiones de fuentes. Las demos usan las dos primeras;
la instancia nueva de OmniRoute usa el digest declarado en runtime.json, cuya
procedencia es la imagen publicada diegosouzapw/omniroute. No se atribuye esa imagen
al fork opcional de sources.json.
scripts/build.run --with-omniroute permite construir aparte el fork seleccionado
con runner-base, pero no instala ni sustituye la instancia existente.

F1 conserva hasta 50 conversaciones en el navegador, sin Thesys Cloud, tokens
frontend ni sincronización entre dispositivos. El historial del navegador es
entrada no confiable; el servidor acepta solo texto user/assistant acotado y
vuelve a consultar las herramientas para fundamentar sus respuestas. Intelligent
conserva su historial local upstream. Las API keys nunca llegan al navegador.

## Comprobación manual breve

1. Verificar DNS y TLS de los dominios sin excepciones de certificado.
2. Abrir F1 y standings/drivers. Preguntar una clasificación real, comprobar
   herramientas, respuesta visual y una segunda pregunta.
3. Recargar y volver a abrir la conversación.
4. Abrir Intelligent UI y generar una UI interactiva.
5. Consultar /v1/models con la API key desde un archivo protegido; verificar
   rechazo sin key.
6. Reiniciar solo las dos unidades de las demos y repetir una pregunta.

Compilación, despliegue, acceso e inferencia se comprueban por separado.

## Activar inferencia después del alta

Entrar a https://omniroute.gnx, configurar un proveedor y sus modelos, y crear una
API key de cliente. En el LXC ejecutar:

    bash /stacks/openui/scripts/configure-client.run

Solicita la key sin eco y el ID de modelo (Enter usa `entry-stack`), escribe apps.env con permisos 0600 y
reinicia únicamente las dos demos. No pegar llaves en el chat ni pasarlas por argv.
Las contraseñas de acceso se entregan en el archivo administrativo protegido
Windows openui-access.txt y permanecen también en secrets/ del guest.

La imagen publicada de OmniRoute usa USER=node. El Quadlet declara :U en su volumen
de datos para que Podman aplique la propiedad correspondiente, conservando contenido.
