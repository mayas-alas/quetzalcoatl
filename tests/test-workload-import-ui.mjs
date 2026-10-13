import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const html = read('apps/web-app/index.html');
const app = read('apps/web-app/app.js');
const css = read('apps/web-app/styles.css');
for (const selector of [':root', 'body', 'main', '.card', '.button', '.power-toggle', '.import-actions']) {
  assert.ok(css.includes(`${selector}{`), `missing base/presentation style: ${selector}`);
}
assert.match(css, /--bg:#07111f/);
assert.match(css, /\[hidden\]\{display:none!important\}/);

assert.match(html, /id="import-folder"[^>]+href="gnx:\/\/workloads\/import\/folder"/);
assert.match(html, /id="import-zip"[^>]+href="gnx:\/\/workloads\/import\/zip"/);
assert.match(html, /selección abre GnX instalado/i);
assert.match(html, /no recibe rutas locales ni secretos/i);
assert.match(html, /id="ai-logs"[^>]+href="gnx:\/\/workloads\/logs\/ai"/);
assert.match(html, /id="ai-toggle"[^>]+href="gnx:\/\/workloads\/ai\/start"/);
assert.match(html, /Content-Security-Policy/);

assert.match(app, /const isTrustedAppOrigin = window\.location\.protocol === 'https:' && window\.location\.hostname === 'app\.gnx'/);
assert.match(app, /if \(!isTrustedAppOrigin\)[\s\S]*querySelectorAll\('a\[href\^="gnx:\/\/"\]'\)/);
assert.doesNotMatch(app, /window\.open|location\.href\s*=|fetch\([^)]*gnx:/);
assert.doesNotMatch(app, /innerHTML/);
assert.match(app, /renderAiWorkload\(ai\)/);
assert.match(app, /ai\.state === 'running' && ai\.service === 'healthy'/);
assert.match(app, /ai\.mesh === true/);
assert.match(app, /serviceEntries\.every\(\(\[, value\]\) => value === true\)/);
assert.match(app, /Array\.isArray\(ai\.events\)[\s\S]*?\.slice\(0, 32\)/);
assert.match(app, /workloads\.slice\(0, 32\)/);
assert.match(app, /aiEvents\.replaceChildren/);
assert.match(app, /item\.textContent = safeText\(event/);
assert.match(app, /errorCode \? `Código: \$\{errorCode\}`/);

// Absent/failed observations must remain observations, never a fabricated ready claim.
assert.doesNotMatch(app, /ai[^\n]*textContent[^\n]*(?:listo|ready)/i);

// Exercise the actual installed frontend source in memory, not only regexes.
function fixture(trusted = true) {
  const nodes = new Map();
  const element = () => ({ textContent: '', hidden: false, children: [], attributes: {}, style: {},
    classList: { toggle() {} }, addEventListener() {}, reset() {},
    setAttribute(name, value) { this.attributes[name] = value; },
    removeAttribute(name) { delete this[name]; delete this.attributes[name]; },
    replaceChildren(...children) { this.children = children; },
  });
  const get = selector => {
    if (!nodes.has(selector)) nodes.set(selector, element());
    return nodes.get(selector);
  };
  for (const [, id, href] of html.matchAll(/<a\s[^>]*id="([^"]+)"[^>]*href="([^"]+)"/g)) get(`#${id}`).href = href;
  const native = [...nodes.values()].filter(node => node.href?.startsWith('gnx://'));
  const context = vm.createContext({
    document: { querySelector: get, querySelectorAll: () => native, createElement: element },
    window: { location: { protocol: 'https:', hostname: trusted ? 'app.gnx' : 'evil.invalid' },
      setInterval() {}, clearInterval() {}, addEventListener() {} },
    navigator: {}, fetch: async () => ({ ok: false, status: 503 }),
  });
  vm.runInContext(app, context);
  return { get, native, render: data => vm.runInContext(`renderAiWorkload(${JSON.stringify(data)})`, context),
    status: data => vm.runInContext(`renderStatus(${JSON.stringify(data)})`, context) };
}
const view = fixture();
for (const observation of [undefined, {}, { services: null }, { state: 'running', service: 'healthy', mesh: false, services: { voice: true } },
  { state: 'ERROR', service: 'unknown', error_code: 'WORKLOAD_STATUS_UNAVAILABLE' }]) {
  view.render(observation);
  assert.notEqual(view.get('#ai-dot').className, 'dot pass');
}
view.render({ state: 'running', service: 'healthy', mesh: true, services: { voice: true, tls: true } });
assert.equal(view.get('#ai-dot').className, 'dot pass');
assert.equal(view.get('#ai-toggle').href, 'gnx://workloads/ai/stop');
view.render({ state: 'running', service: 'attention', mesh: true, services: Object.fromEntries(Array.from({ length: 40 }, (_, i) => [`svc-${i}`, false])),
  error_code: 'MESH_TLS_CERT_REQUIRED', events: Array(40).fill('token=private-fixture <img src=x onerror=alert(1)>\n') });
assert.equal(view.get('#ai-services').children.length, 32);
assert.equal(view.get('#ai-events').children.length, 32);
assert.ok(view.get('#ai-events').children.every(item => !item.textContent.includes('private-fixture')));
assert.equal(view.get('#ai-error').hidden, false);
assert.ok(fixture(false).native.every(node => node.href === undefined), 'untrusted origin retained native intents');
const shared = { workload: 'rustfs', state: 'running', service: 'healthy', services: { 'continuity-business': false } };
view.status({ state: 'NodeReady', checks: [{ result: 'pass' }], workloads: [shared] });
assert.equal(view.get('#business-link').hidden, true);
shared.services['continuity-business'] = true;
view.status({ state: 'NodeReady', checks: [{ result: 'pass' }], workloads: [shared] });
assert.equal(view.get('#business-link').hidden, false);
assert.equal(view.get('#business-dot').className, 'dot pass');
console.log('PASS: fixed native import links, origin boundary, real renderer error/health cases, null services and bounded text-only events.');
