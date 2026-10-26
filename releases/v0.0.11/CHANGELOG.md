# GnX Core — v0.0.11

Fecha: 2026-10-26. Distribución: candidata local; publicación no incluida.

## Añadido

- `gnx tls`: aplicación administrativa explícita del certificado/CRL del paquete
  instalado, mediante el pipe protegido y la identidad Windows dedicada.
- Punto de distribución de CRL en el certificado servidor:
  `http://app.gnx/pki/root.crl`. El gateway existente sirve solo la CRL en HTTP.
- Servicio y timer systemd para firmar la CRL al iniciar el gateway y cada seis
  horas. La lista vence en 24 horas y se publica como DER mediante reemplazo atómico.
- `scripts/clean-release-exes.ps1`: vista previa de instaladores EXE antiguos y
  eliminación solo con `-Apply`. Conserva el último setup con MSI, versión y
  binario coincidentes; no borra MSI ni `releases/v*/gnx.exe`.

## Seguridad

- Conserva la CA, las claves y los SAN existentes; no exige instalar otra raíz.
- Índice y material de firma privados; bloqueo exclusivo compartido entre
  emisión y renovación. No se reinicia ni se reconstruye el índice existente.
- HTTP 80 limitado a las direcciones privadas del gateway y al reenvío Windows
  `127.0.0.1:80`, con ownership comprobado. Las demás rutas HTTP devuelven 404.
- Firma de CRL y cadena del certificado verificadas con OpenSSL. No se usa
  TLS inseguro ni se desactiva la comprobación de revocación.

## Actualización y operación

- Tras instalar, ejecutar `gnx tls` con el target activo. `start` y `status` no
  aplican cambios de certificado ni despiertan el nodo para configurar TLS.
- La operación `gnx tls` no cambia MTU, mesh, datos ni credenciales y reinicia
  solo el gateway, no Proxmox. Instalar el paquete puede requerir una parada y
  reanudación ordenadas del nodo; no se promete una actualización sin interrupción.
- Diagnóstico nativo en `journalctl -u gnx-core-tls-apply`,
  `gnx-core-crl.service`, `gnx-core-crl.timer` y el registro de eventos Windows.

## Limitaciones conocidas

- Los clientes pueden cachear CRL hasta `nextUpdate`; no se promete revocación
  inmediata ni consulta obligatoria por todos los navegadores.
- No añade OCSP ni una API de revocación. Revocar un serial requiere OpenSSL
  bajo el mismo bloqueo de la CA y publicar después la CRL mediante su servicio.
- No atribuye ni corrige por suposición incidentes remotos de DNS, MTU o HTTP/3.
- La desinstalación conservadora completa y la purga separada siguen pendientes.
