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
const rustfsDot = document.querySelector('#rustfs-dot');
const rustfsVersion = document.querySelector('#rustfs-version');
const rustfsState = document.querySelector('#rustfs-state');
const rustfsToggle = document.querySelector('#rustfs-toggle');
const rustfsLink = document.querySelector('#rustfs-link');
const githubDot = document.querySelector('#github-dot');
const githubState = document.querySelector('#github-state');
const githubDetail = document.querySelector('#github-detail');
const githubActionsStatus = document.querySelector('#github-actions-status');
const githubConnect = document.querySelector('#github-connect');
const githubStart = document.querySelector('#github-start');
const githubStop = document.querySelector('#github-stop');

const checkLabels = { pass: 'Correcto', pending: 'Pendiente', attention: 'Requiere atención' };
const githubStateLabels = {
  disconnected: 'GitHub no conectado',
  ready: 'GitHub listo',
  starting: 'Iniciando exit node…',
  running: 'Exit node activo',
  stopping: 'Deteniendo exit node…',
  error: 'Error del exit node',
};
const githubStateClasses = {
  disconnected: 'pending',
  ready: 'pass',
  starting: 'pending',
  running: 'pass',
  stopping: 'pending',
  error: 'attention',
};
const githubActionLabels = {
  connect: 'Conectar GitHub',
  github: 'Conectar GitHub',
  start: 'Iniciar 5 h',
  start_5h: 'Iniciar 5 h',
  start5h: 'Iniciar 5 h',
  connect_github: 'Conectar GitHub',
  github_connect: 'Conectar GitHub',
  stop: 'Detener',
};
const githubLinks = Object.freeze({
  connect: 'gnx://github/connect',
  start: 'gnx://github/start-5h',
  stop: 'gnx://github/stop',
});

// Status is service-provided data. Keep it bounded, text-only, and never use it as markup.
function safeText(value, fallback = '') {
  if (typeof value !== 'string') return fallback;
  return value.replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\b(?:password|secret|token)\s*[:=]\s*[^\s]+/gi, '[redacted]')
    .replace(/\btskey-[^\s]+/gi, '[redacted]')
    .replace(/\s+/g, ' ').trim().slice(0, 240);
}

function stateKey(value) {
  const key = safeText(value).toLowerCase().replace(/[\s-]+/g, '_');
  if (['disconnected', 'not_connected', 'unconnected', 'none'].includes(key)) return 'disconnected';
  if (['ready', 'connected', 'available'].includes(key)) return 'ready';
  if (['starting', 'pending', 'provisioning', 'queued'].includes(key)) return 'starting';
  if (['running', 'active'].includes(key)) return 'running';
  if (['stopping', 'stopped_pending', 'terminating'].includes(key)) return 'stopping';
  if (['error', 'failed', 'failure'].includes(key)) return 'error';
  return 'disconnected';
}

function sourceState(source) {
  if (typeof source === 'string') return source;
  if (!source || typeof source !== 'object') return '';
  return source.state || source.status || source.phase || '';
}

function sourceMessage(source) {
  if (!source || typeof source !== 'object') return '';
  return safeText(source.message || source.detail || source.error_message || source.error);
}

function githubStatusSource(status) {
  if (!status || typeof status !== 'object') return { state: 'disconnected' };
  const integrations = status.integrations && typeof status.integrations === 'object' ? status.integrations : {};
  const actions = status.actions && typeof status.actions === 'object' && !Array.isArray(status.actions)
    ? status.actions
    : {};
  const actionList = Array.isArray(status.actions) ? status.actions : [];
  const githubAction = actionList.find(item => {
    const name = safeText(item?.action || item?.name || item?.id).toLowerCase();
    return name.includes('github') || name.includes('exit');
  });
  const namedAction = Object.entries(actions).find(([name]) => {
    const key = safeText(name).toLowerCase();
    return key.includes('github') || key.includes('exit');
  });
  return status.github_exit_node || status.githubExitNode || status.exit_node || status.exitNode
    || integrations.github_exit_node || integrations.github || actions.github_exit_node || actions.github
    || (actions.state || actions.status ? actions : null) || githubAction || namedAction?.[1]
    || { state: 'disconnected' };
}

function actionEntries(source, status) {
  const actions = source && typeof source === 'object' ? source.actions : null;
  const statusActions = status && typeof status === 'object' ? status.actions : null;
  if (Array.isArray(actions)) return actions.map((item, index) => [String(index), item]);
  if (actions && typeof actions === 'object') return Object.entries(actions);
  if (Array.isArray(statusActions)) {
    return statusActions
      .filter(item => item && typeof item === 'object')
      .map((item, index) => [item.action || item.name || String(index), item]);
  }
  if (statusActions && typeof statusActions === 'object') return Object.entries(statusActions);
  return [];
}

function renderGithubActions(source, status) {
  const entries = actionEntries(source, status).slice(0, 8);
  if (!entries.length) {
    githubActionsStatus.hidden = true;
    githubActionsStatus.replaceChildren();
    return;
  }
  githubActionsStatus.hidden = false;
  githubActionsStatus.replaceChildren(...entries.map(([name, value]) => {
    const item = document.createElement('li');
    const key = safeText(name, 'acción').toLowerCase().replace(/[\s-]+/g, '_');
    const label = githubActionLabels[key] || safeText(name, 'Acción');
    const state = stateKey(sourceState(value));
    item.className = `action-status-item ${githubStateClasses[state]}`;
    const detail = sourceMessage(value);
    item.textContent = `${label}: ${githubStateLabels[state]}${detail ? ` — ${detail}` : ''}`;
    return item;
  }));
}

function renderGithub(status) {
  const source = githubStatusSource(status);
  const state = stateKey(sourceState(source));
  const detail = sourceMessage(source);
  githubDot.className = `dot ${githubStateClasses[state]}`;
  githubState.textContent = githubStateLabels[state];
  githubDetail.textContent = detail || (state === 'disconnected'
    ? 'Conecta una cuenta de GitHub para habilitar una sesión de 5 horas.'
    : state === 'error'
      ? 'Revisa el servicio Windows e inténtalo de nuevo.'
      : 'El servicio Windows confirma cada acción antes de actuar.');
  githubConnect.hidden = !['disconnected', 'error'].includes(state);
  githubStart.hidden = state !== 'ready';
  githubStop.hidden = !['starting', 'running'].includes(state);
  githubConnect.href = githubLinks.connect;
  githubStart.href = githubLinks.start;
  githubStop.href = githubLinks.stop;
  renderGithubActions(source, status);
}

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
  renderGithub(safeStatus);
}

async function refreshStatus() {
  try {
    const response = await fetch('./status.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('status unavailable');
    renderStatus(await response.json());
  } catch (_) {
    statusTitle.textContent = 'Estado no disponible';
    statusMessage.textContent = 'No se pudo leer el último estado confirmado del nodo.';
    renderGithub({ github_exit_node: { state: 'error', message: 'No se pudo leer el estado confirmado.' } });
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
