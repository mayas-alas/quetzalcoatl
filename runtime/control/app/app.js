const clock = document.querySelector('#clock');
clock.textContent = new Intl.DateTimeFormat('es', {hour: '2-digit', minute: '2-digit', timeZoneName: 'short'}).format(new Date());
const button = document.querySelector('#check');
const result = document.querySelector('#result');
button.addEventListener('click', async () => {
  button.disabled = true;
  result.textContent = 'Comprobando…';
  try {
    const response = await fetch('/', {cache: 'no-store', signal: AbortSignal.timeout(8000)});
    if (!response.ok) throw new Error('HTTP ' + response.status);
    result.textContent = 'Esta página responde por HTTPS · ' + new Date().toLocaleTimeString('es');
  } catch {
    result.textContent = 'Sin respuesta. Comprueba tu conexión Tailscale.';
  } finally {
    button.disabled = false;
  }
});
