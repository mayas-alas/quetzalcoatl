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
const servicesToggle = document.querySelector('#services-toggle');
const servicesState = document.querySelector('#services-state');
const rustfsLink = document.querySelector('#rustfs-link');
const aiDot = document.querySelector('#ai-dot');
const aiToggle = document.querySelector('#ai-toggle');
const aiState = document.querySelector('#ai-state');
const aiService = document.querySelector('#ai-service');
const aiMesh = document.querySelector('#ai-mesh');
const aiServices = document.querySelector('#ai-services');
const aiError = document.querySelector('#ai-error');
const aiEvents = document.querySelector('#ai-events');
const workloadsSection = document.querySelector('#workloads-section');
const workloadsList = document.querySelector('#workloads-list');
const githubForm = document.querySelector('#github-config');
const githubState = document.querySelector('#github-state');

const checkLabels = { pass: 'Correcto', pending: 'Pendiente', attention: 'Requiere atención' };

if (!isTrustedAppOrigin) {
  document.querySelectorAll('a[href^="gnx://"]').forEach(link => link.removeAttribute('href'));
}


// Status is service-provided data. Keep it bounded, text-only, and never use it as markup.
function safeText(value, fallback = '') {
  if (typeof value !== 'string') return fallback;
  return value.replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\b(?:password|secret|token)\s*[:=]\s*[^\s]+/gi, '[redacted]')
    .replace(/\btskey-[^\s]+/gi, '[redacted]')
    .replace(/\s+/g, ' ').trim().slice(0, 240);
}

function observedText(value, fallback = 'No observado') {
  return typeof value === 'string' && safeText(value) ? safeText(value) : fallback;
}

function observedBoolean(value) {
  return value === true ? 'Sí' : value === false ? 'No' : 'No observado';
}

function boundedServiceEntries(workload) {
  const services = workload?.services && typeof workload.services === 'object' && !Array.isArray(workload.services)
    ? Object.entries(workload.services)
    : [];
  return services.slice(0, 32);
}

function renderAiWorkload(workload) {
  const ai = workload && typeof workload === 'object' ? workload : {};
  const serviceEntries = boundedServiceEntries(ai);
  const errorCode = safeText(ai.error_code);
  const state = observedText(ai.state);
  const service = observedText(ai.service);
  const hasFalseService = serviceEntries.some(([, value]) => value === false);
  const observedHealthy = ai.state === 'running' && ai.service === 'healthy'
    && ai.mesh === true && serviceEntries.length > 0
    && serviceEntries.every(([, value]) => value === true) && !errorCode;
  const attention = Boolean(errorCode) || ai.mesh === false || hasFalseService
    || ai.state === 'failed' || ai.service === 'unhealthy';

  aiState.textContent = `Estado observado: ${state}`;
  aiService.textContent = service;
  aiMesh.textContent = observedBoolean(ai.mesh);
  aiError.hidden = !errorCode;
  aiError.textContent = errorCode ? `Código: ${errorCode}` : '';
  aiDot.className = `dot ${observedHealthy ? 'pass' : attention ? 'attention' : 'pending'}`;

  const running = ai.state === 'running';
  aiToggle.href = running ? 'gnx://workloads/ai/stop' : 'gnx://workloads/ai/start';
  aiToggle.setAttribute('aria-checked', String(running));
  aiToggle.setAttribute('aria-label', `${running ? 'Apagar' : 'Encender'} workload AI`);
  aiToggle.classList.toggle('on', running);

  aiServices.replaceChildren(...serviceEntries.map(([name, value]) => {
    const item = document.createElement('li');
    item.textContent = `${observedText(name, 'Servicio')}: ${observedBoolean(value)}`;
    return item;
  }));

  const events = Array.isArray(ai.events) ? ai.events.filter(event => typeof event === 'string').slice(0, 32) : [];
  aiEvents.replaceChildren(...events.map(event => {
    const item = document.createElement('li');
    item.textContent = safeText(event, 'Evento no disponible');
    return item;
  }));
}

function renderWorkloadRows(workloads) {
  const rows = workloads.slice(0, 32).map((workload, index) => {
    const item = document.createElement('li');
    const name = observedText(workload?.workload, `Workload ${index + 1}`);
    const state = observedText(workload?.state);
    const service = observedText(workload?.service);
    item.textContent = `${name}: estado ${state}; servicio ${service}`;
    return item;
  });
  workloadsList.replaceChildren(...rows);
  workloadsSection.hidden = rows.length === 0;
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
  renderWorkloadRows(workloads);
  // 'rustfs' is the legacy IPC ownership key for the shared services guest;
  // it is not the semantic name of the machine or identity/network services.
  const rustfs = workloads.find(workload => workload && workload.workload === 'rustfs');
  const ai = workloads.find(workload => workload && workload.workload === 'ai');
  renderAiWorkload(ai);
  const running = rustfs?.state === 'running';
  const healthyWorkload = running && rustfs?.service === 'healthy';
  rustfsDot.className = `dot ${healthyWorkload ? 'pass' : running ? 'attention' : 'pending'}`;
  rustfsVersion.textContent = rustfs?.version ? `RustFS ${safeText(rustfs.version)}` : 'Almacenamiento de objetos';
  rustfsState.textContent = healthyWorkload
    ? 'Almacenamiento operativo.'
    : running
      ? 'El almacenamiento requiere atención.'
      : rustfs?.state === 'off' || rustfs?.state === 'stopped'
        ? 'Almacenamiento detenido; los datos permanecen guardados.'
        : 'Estado del almacenamiento no disponible; revisa el diagnóstico.';
  rustfsLink.hidden = !healthyWorkload;
  servicesState.textContent = running ? 'LXC compartido encendido.'
    : rustfs?.state === 'off' || rustfs?.state === 'stopped' ? 'LXC compartido apagado.'
      : 'No se pudo observar el estado del LXC compartido.';
  servicesToggle.href = running ? 'gnx://workloads/private-services/stop' : 'gnx://workloads/private-services/start';
  servicesToggle.setAttribute('aria-checked', String(running));
  servicesToggle.setAttribute('aria-label', `${running ? 'Apagar' : 'Encender'} servicios privados`);
  servicesToggle.classList.toggle('on', running);
  const businessReady = running && rustfs?.services?.['continuity-business'] === true;
  document.querySelector('#business-dot').className = `dot ${businessReady ? 'pass' : running ? 'attention' : 'pending'}`;
  document.querySelector('#business-state').textContent = businessReady
    ? 'Memoria de negocio operativa; réplica sin pendientes.'
    : running ? 'Continuity requiere atención; consulta el diagnóstico.' : 'Máquina de servicios privados detenida.';
  document.querySelector('#business-link').hidden = !businessReady;

  for (const [name, result, label] of [
    ['pocket', rustfs?.pocket_id, 'Pocket ID'],
    ['netbird', rustfs?.netbird, 'NetBird'],
  ]) {
    const dot = document.querySelector(`#${name}-dot`);
    const state = document.querySelector(`#${name}-state`);
    const link = document.querySelector(`#${name}-link`);
    if (!dot || !state || !link) continue;
    const ready = healthy && running && result === 'healthy';
    dot.className = `dot ${ready ? 'pass' : running ? 'attention' : 'pending'}`;
    state.textContent = ready ? `${label} operativo en el LXC privado.`
      : running ? `${label} requiere atención; no se afirma que esté listo.`
        : 'LXC apagado; los datos permanecen guardados.';
    link.hidden = !ready;
  }
}

let sessionEnded = false;
let statusTimer;
function clearPrivateView() {
  sessionEnded = true;
  if (statusTimer) window.clearInterval(statusTimer);
  githubForm.reset();
  document.querySelector('main').hidden = true;
}
async function logout(event) {
  event.preventDefault();
  clearPrivateView();
  try {
    const response = await fetch('/logout', { method: 'POST', credentials: 'same-origin', cache: 'no-store', redirect: 'follow' });
    if (!response.ok) throw new Error('logout failed');
    window.location.replace('/signed-out');
  } catch (_) {
    // Never claim the server session was deleted after a network failure.
    document.querySelector('#session-message').textContent = 'No se pudo confirmar el cierre de sesión. Reconecta y vuelve a pulsar Cerrar sesión.';
  }
}
const logoutButton = document.querySelector('#logout');
if (isTrustedAppOrigin) logoutButton.addEventListener('click', logout);

async function refreshStatus() {
  if (sessionEnded) return;
  try {
    const response = await fetch('./status.json', { cache: 'no-store', redirect: 'manual' });
    if (response.status === 401 || response.status === 403 || response.type === 'opaqueredirect'
        || (response.ok && !response.headers.get('content-type')?.includes('application/json'))) {
      clearPrivateView();
      window.location.replace('/oauth2/sign_in?rd=%2F');
      return;
    }
    if (!response.ok) throw new Error('status unavailable');
    const status = await response.json();
    if (!sessionEnded) renderStatus(status);
  } catch (_) {
    statusTitle.textContent = 'Estado no disponible';
    statusMessage.textContent = 'No se pudo leer el último estado confirmado del nodo.';

  }
}

if (isTrustedAppOrigin) {
  void refreshStatus();
  statusTimer = window.setInterval(() => void refreshStatus(), 15000);
}

if (isTrustedAppOrigin) {
  // BFCache must not restore private DOM after logout or session expiry.
  window.addEventListener('pagehide', () => { clearPrivateView(); });
  window.addEventListener('pageshow', event => { if (event.persisted) window.location.reload(); });
}

if (isTrustedAppOrigin && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    // Only update existing legacy registrations. Registering the retirement
    // worker anew on every load would unregister+navigate in a reload loop.
    void navigator.serviceWorker.getRegistrations().then(registrations => Promise.all(
      registrations.filter(registration => registration.scope === 'https://app.gnx/')
        .map(registration => registration.update())
    ));
  });
}
