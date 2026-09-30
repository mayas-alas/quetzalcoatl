// PWA de estado y acceso, sin APIs administrativas.
const isTrustedAppOrigin = window.location.protocol === 'https:' && window.location.hostname === 'app.gnx';
const statusTitle = document.querySelector('#status-title');
const statusMessage = document.querySelector('#status-message');
const statusBar = document.querySelector('#status-bar');
const statusProgress = document.querySelector('.progress');
const statusPercent = document.querySelector('#status-percent');
const statusChecks = document.querySelector('#status-checks');
const statusError = document.querySelector('#status-error');
const statusDot = document.querySelector('#status-dot');
const computeLink = document.querySelector('#compute-link');

const checkLabels = { pass: 'Correcto', pending: 'Pendiente', attention: 'Requiere atención' };

function renderStatus(status) {
  const percent = Math.max(0, Math.min(100, Number(status.percent) || 0));
  const checks = Array.isArray(status.checks) ? status.checks : [];
  const ready = status.state === 'NodeReady';
  const healthy = ready && checks.length > 0 && checks.every(check => check.result === 'pass');
  statusTitle.textContent = healthy ? 'Nodo operativo' : 'Estado del nodo';
  statusMessage.textContent = status.message || 'No hay un mensaje de estado disponible.';
  statusBar.style.width = `${percent}%`;
  statusProgress.setAttribute('aria-valuenow', String(percent));
  statusPercent.textContent = `${percent}% completado`;
  statusDot.className = `dot ${healthy ? 'pass' : status.error_code || checks.some(check => check.result === 'attention') ? 'attention' : 'pending'}`;
  statusError.hidden = !status.error_code;
  statusError.textContent = status.error_code ? `Código: ${status.error_code}` : '';
  computeLink.hidden = !healthy;
  statusChecks.replaceChildren(...checks.map(check => {
    const item = document.createElement('li');
    const result = checkLabels[check.result] || checkLabels.pending;
    item.className = `check ${check.result || 'pending'}`;
    item.textContent = `${check.label}: ${result}`;
    return item;
  }));
}

async function refreshStatus() {
  try {
    const response = await fetch('./status.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('status unavailable');
    renderStatus(await response.json());
  } catch (_) {
    statusTitle.textContent = 'Estado no disponible';
    statusMessage.textContent = 'No se pudo leer el último estado confirmado del nodo.';
  }
}

if (isTrustedAppOrigin) {
  void refreshStatus();
  window.setInterval(() => void refreshStatus(), 15000);
}

if (isTrustedAppOrigin && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    void navigator.serviceWorker.register('./sw.js', { scope: './' });
  });
}
