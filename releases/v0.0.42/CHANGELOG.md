# GnX Core 0.0.42

- Añade `gnx deploy --stack kestra`: Kestra 2.0.11 y PostgreSQL 16.16 mediante Quadlets en un LXC no privilegiado, con imágenes fijadas por digest, credenciales persistentes y UI solo en loopback del guest.
- Conserva configuración y recibos mediante las fronteras Rust existentes; sin socket del host ni activación al instalar el producto. Empaquetado no acredita aceptación runtime.
