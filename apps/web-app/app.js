// Estado del nodo y configuración GitHub directa, sin control Windows para GitHub.
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
const rustfsDot = document.querySelector('#rustfs-dot');
const rustfsVersion = document.querySelector('#rustfs-version');
const rustfsState = document.querySelector('#rustfs-state');
const rustfsToggle = document.querySelector('#rustfs-toggle');
const rustfsLink = document.querySelector('#rustfs-link');
const githubForm = document.querySelector('#github-config');
const githubState = document.querySelector('#github-state');

const checkLabels = { pass: 'Correcto', pending: 'Pendiente', attention: 'Requiere atención' };


// Status is service-provided data. Keep it bounded, text-only, and never use it as markup.
function safeText(value, fallback = '') {
  if (typeof value !== 'string') return fallback;
  return value.replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\b(?:password|secret|token)\s*[:=]\s*[^\s]+/gi, '[redacted]')
    .replace(/\btskey-[^\s]+/gi, '[redacted]')
    .replace(/\s+/g, ' ').trim().slice(0, 240);
}

// The token lives only in this operation: no persistence, native bridge or OAuth.
async function configureGithub(event) {
  event.preventDefault();
  if (!isTrustedAppOrigin || !githubForm.reportValidity()) return;
  const fields = githubForm.elements;
  const button = fields.namedItem('submit');
  if (button.disabled) return;
  let token = fields.namedItem('key').value.trim();
  fields.namedItem('key').value = '';
  button.disabled = true;
  let repositoryCreated = false;
  let repositoryPath = '';
  try {
    const name = fields.namedItem('repository').value.trim();
    const file = fields.namedItem('workflow').files[0];
    if (!/^[a-zA-Z0-9_-][a-zA-Z0-9_.-]{0,99}$/.test(name)
        || !file || !/^[a-zA-Z0-9_-][a-zA-Z0-9_.-]*\.ya?ml$/.test(file.name)
        || file.size === 0 || file.size > 262144) throw new Error('Configuración inválida.');
    const content = btoa(Array.from(new Uint8Array(await file.arrayBuffer()), byte => String.fromCharCode(byte)).join(''));
    const api = async (path, method = 'GET', body) => {
      const response = await fetch(`https://api.github.com${path}`, {
        method, cache: 'no-store', credentials: 'omit', redirect: 'error',
        headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json',
          'X-GitHub-Api-Version': '2022-11-28', 'Content-Type': 'application/json' },
        ...(body ? { body: JSON.stringify(body) } : {}),
      });
      // Never echo API bodies: they may contain private account data.
      if (!response.ok) throw new Error(`GitHub HTTP ${response.status}.`);
      return response;
    };
    githubState.textContent = 'Comprobando permisos…';
    const identity = await api('/user');
    const scopes = (identity.headers.get('x-oauth-scopes') || '').split(',').map(scope => scope.trim());
    if (!scopes.includes('repo') || !scopes.includes('workflow')) {
      throw new Error('Se requiere un token clásico con permisos repo y workflow.');
    }
    // Only create a new private repository; never overwrite an existing workflow.
    const response = await api('/user/repos', 'POST', { name, private: true, auto_init: true });
    const repository = await response.json();
    repositoryCreated = true;
    if (!/^[a-zA-Z0-9_-]+$/.test(repository.owner?.login || '') || repository.name !== name
        || repository.private !== true || typeof repository.default_branch !== 'string') {
      throw new Error('Respuesta de repositorio inválida.');
    }
    repositoryPath = `/repos/${repository.owner.login}/${name}`;
    await api(`${repositoryPath}/contents/.github/workflows/${encodeURIComponent(file.name)}`, 'PUT', {
      message: 'Configure GitHub Actions', content, branch: repository.default_branch,
    });
    githubState.textContent = `Repositorio privado ${repository.owner.login}/${name} configurado. Añade los secrets requeridos en GitHub antes de ejecutar Actions.`;
  } catch (error) {
    githubState.textContent = `${repositoryCreated ? 'Repositorio creado; falta instalar el workflow. Revisa el repositorio en GitHub. ' : ''}${safeText(error.message, 'No se pudo configurar GitHub.')}`;
  } finally {
    token = '';
    fields.namedItem('key').value = '';
    button.disabled = false;
  }
}

if (isTrustedAppOrigin) githubForm.addEventListener('submit', configureGithub);

function renderStatus(status) {
  const safeStatus = status && typeof status === 'object' ? status : {};
  const percent = Math.max(0, Math.min(100, Number(safeStatus.percent) || 0));
  const checks = Array.isArray(safeStatus.checks) ? safeStatus.checks : [];
  const ready = safeStatus.state === 'NodeReady';
  const healthy = ready && checks.length > 0 && checks.every(check => check && check.result === 'pass');
  statusTitle.textContent = healthy ? 'Nodo operativo' : 'Estado del nodo';
  statusMessage.textContent = safeText(safeStatus.message, 'No hay un mensaje de estado disponible.');
  statusBar.style.width = `${percent}%`;
  statusProgress.setAttribute('aria-valuenow', String(percent));
  statusPercent.textContent = `${percent}% completado`;
  const nodeClass = safeStatus.error_code || checks.some(check => check && check.result === 'attention')
    ? 'attention'
    : healthy ? 'pass' : 'pending';
  statusDot.className = `dot ${nodeClass}`;
  const errorCode = safeText(safeStatus.error_code);
  statusError.hidden = !errorCode;
  statusError.textContent = errorCode ? `Código: ${errorCode}` : '';
  computeLink.hidden = !healthy;
  statusChecks.replaceChildren(...checks.slice(0, 32).map(check => {
    const item = document.createElement('li');
    const result = checkLabels[check?.result] || checkLabels.pending;
    item.className = `check ${checkLabels[check?.result] ? check.result : 'pending'}`;
    item.textContent = `${safeText(check?.label, 'Comprobación')}: ${result}`;
    return item;
  }));

  const workloads = Array.isArray(safeStatus.workloads) ? safeStatus.workloads : [];
  const rustfs = workloads.find(workload => workload && workload.workload === 'rustfs');
  const running = rustfs?.state === 'running';
  const healthyWorkload = running && rustfs?.service === 'healthy';
  rustfsDot.className = `dot ${healthyWorkload ? 'pass' : running ? 'attention' : 'pending'}`;
  rustfsVersion.textContent = rustfs?.version ? `RustFS ${safeText(rustfs.version)}` : 'Almacenamiento de objetos';
  rustfsState.textContent = healthyWorkload
    ? 'Máquina encendida y servicio operativo.'
    : running
      ? 'Máquina encendida; el servicio requiere atención.'
      : 'Máquina apagada; los datos permanecen guardados.';
  rustfsLink.hidden = !healthyWorkload;
  rustfsToggle.href = running ? 'gnx://workloads/rustfs/stop' : 'gnx://workloads/rustfs/start';
  rustfsToggle.setAttribute('aria-checked', String(running));
  rustfsToggle.setAttribute('aria-label', `${running ? 'Apagar' : 'Encender'} RustFS`);
  rustfsToggle.classList.toggle('on', running);

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
