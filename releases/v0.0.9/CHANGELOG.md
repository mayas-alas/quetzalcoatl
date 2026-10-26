# GnX Core v0.0.9

- Fija la dirección interna de compute en el bridge de la base sellada para
  conservar el destino del forwarding de Proxmox al recrear su contenedor.
- Conserva la publicación de `8006` solo en loopback, con acceso HTTPS mediante
  Caddy; no añade puertos externos, servicios ni rutinas de reinicio.
- Mantiene la observación nativa del target y la distinción entre última fase
  publicada y salud actual introducidas en v0.0.8.

La actualización conserva datos, credenciales, CA e identidad mesh. El acceso
desde otro dispositivo y la instalación desde cero se verifican por separado.
