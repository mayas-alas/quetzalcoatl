import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const read = async relative => readFile(join(root, relative), 'utf8');

const routes = await read('payload/node/gateway/routes.conf');
assert.match(routes, /https:\/\/app\.gnx[\s\S]*tls \/etc\/gnx\/ca\/gateway-cert\.pem \/etc\/gnx\/ca\/gateway-key\.pem/);
assert.match(routes, /https:\/\/compute\.gnx[\s\S]*reverse_proxy https:\/\/127\.0\.0\.1:8006/);
assert.match(routes, /tls_trust_pool file \/etc\/gnx\/ca\/backend-ca\.pem/);
assert.match(routes, /tls_server_name compute\.gnx/);
assert.doesNotMatch(routes, /tls_insecure_skip_verify|tls_server_name\s+127\.0\.0\.1/);
assert.match(routes, /protocols h1 h2/);
assert.doesNotMatch(routes, /protocols[^\n]*h3/);
assert.equal((routes.match(/Alt-Svc "clear"/g) ?? []).length, 5);
assert.match(routes, /https:\/\/rustfs\.gnx[\s\S]*reverse_proxy https:\/\/rustfs\.mesh\.gnx:443/);
assert.match(routes, /https:\/\/pocket\.gnx[\s\S]*reverse_proxy http:\/\/pocket\.mesh\.gnx:1411/);
assert.match(routes, /https:\/\/netbird\.gnx[\s\S]*reverse_proxy http:\/\/netbird\.mesh\.gnx:8080/);
assert.match(routes, /h2c:\/\/netbird\.mesh\.gnx:8081/);
assert.match(routes, /resolvers 127\.0\.0\.1:5353/);
assert.match(routes, /tls_server_name rustfs\.gnx/);
assert.match(routes, /header_up Host \{http\.request\.host\}/);
assert.match(routes, /tls \/etc\/gnx\/ca\/rustfs-cert\.pem \/etc\/gnx\/ca\/rustfs-key\.pem/);
assert.match(routes, /tls_trust_pool file \/etc\/gnx\/ca\/gateway-ca\.pem/);
assert.match(routes, /@status path \/status\.json/);
assert.match(routes, /header @status Cache-Control "no-store"/);

const ingress = await read('payload/node/network/ingress.nft');
assert.match(ingress, /policy drop/);
assert.match(ingress, /^\s*ct state established,related accept\s*$/m);
assert.match(ingress, /iifname "lo" accept/);
assert.match(ingress, /iifname "tailscale0" udp dport 5353 accept/);
assert.match(ingress, /iifname "tailscale0" tcp dport 5353 accept/);
assert.match(ingress, /iifname "tailscale0" tcp dport 443 accept/);
assert.doesNotMatch(ingress, /dport 8006/);

const acceptRules = ingress.split(/\r?\n/).map(line => line.trim()).filter(line => line.endsWith('accept'));
assert.deepEqual(acceptRules.filter(rule => rule.includes('ct state')), ['ct state established,related accept']);
for (const rule of acceptRules.filter(rule => !rule.includes('ct state'))) {
  assert.match(rule, /^(?:iifname "(?:lo|tailscale0)"(?: (?:udp|tcp) dport (?:443|5353))?|iifname "eth0" ip saddr 172\.16\.0\.0\/12 tcp dport 443) accept$/,
    `new inbound accept rule is not limited to approved local/tailnet sources: ${rule}`);
}

const webFiles = ['index.html', 'app.js', 'sw.js', 'manifest.webmanifest'];
const web = await Promise.all(webFiles.map(file => read(`apps/web-app/${file}`)));
const webText = web.join('\n');
assert.doesNotMatch(webText, /127\.0\.0\.1|localhost|pipe|localStorage|sessionStorage/i);
assert.match(web[0], /Content-Security-Policy/);
assert.match(web[1], /protocol === 'https:'/);
assert.match(web[1], /hostname === 'app\.gnx'/);
assert.match(web[1], /fetch\('\.\/status\.json', \{ cache: 'no-store' \}\)/);
assert.match(web[1], /window\.setInterval\(\(\) => void refreshStatus\(\), 15000\)/);
assert.match(web[1], /safeStatus\.state === 'NodeReady'/);
assert.match(web[2], /event\.request\.method !== 'GET'/);
assert.match(web[2], /url\.origin !== self\.location\.origin/);
assert.match(web[2], /cache\.addAll/);
assert.match(web[2], /gnx-app-v\d+/);
assert.doesNotMatch(web[2], /compute\.gnx|Authorization|localStorage|sessionStorage/i);
assert.match(web[3], /"start_url":"\.\/"/);
assert.match(web[3], /"scope":"\.\/"/);

const app = web[1];
assert.doesNotMatch(app, /JoinMesh|tailscale_auth_key|proxmox_password|onboarding/i);
assert.match(web[0], /connect-src 'self' https:\/\/api\.github\.com/);
assert.match(routes, /connect-src 'self' https:\/\/api\.github\.com/);
assert.match(web[0], /name="key" type="password"/);
assert.match(web[0], /name="confirm" type="checkbox" required/);
assert.doesNotMatch(webText, /gnx:\/\/github|github-start|github-stop|github-actions-status|renderGithub/);
assert.match(app, /credentials: 'omit', redirect: 'error'/);
assert.match(app, /scopes\.includes\('repo'\).*scopes\.includes\('workflow'\)/);
assert.match(app, /private: true, auto_init: true/);
const nativeLinks = webText.match(/gnx:\/\/[^'"\s]+/g) ?? [];
assert.ok(nativeLinks.length > 0);
for (const link of nativeLinks) assert.match(link, /^gnx:\/\/workloads\/rustfs\/(?:start|stop)$/);
assert.match(await read('payload/node/install.run'), /--compute-secret-file/);
assert.match(await read('payload/node/install.run'), /PROXMOX_PASSWORD/);
const compute = await read('payload/node/services/compute.container');
assert.match(compute, /^EnvironmentFile=\/var\/lib\/gnx-node\/secrets\/compute\.env$/m);
assert.match(compute, /^PublishPort=127\.0\.0\.1:8006:8006$/m);
assert.doesNotMatch(compute, /^PublishPort=(?!127\.0\.0\.1:8006:8006)/m, 'compute must not publish 8006 publicly');
assert.doesNotMatch(compute, /^Network=host$/m, 'compute must not use host networking');
const runtimeInstall = await read('payload/node/install.run');
assert.doesNotMatch(runtimeInstall, /Environment=PROXMOX_PASSWORD=|Environment=TS_AUTHKEY=/);
assert.match(runtimeInstall, /mesh.*pending/);
assert.match(runtimeInstall, /private-ca\.cert\.pem/);
assert.match(runtimeInstall, /private-ca\.key\.pem/);

// Exercise the actual frontend in memory: no fixtures or temporary files.
async function githubCase({ scopes = 'repo, workflow', failure = 0, trusted = true, valid = true } = {}) {
  const calls = [];
  const controls = {
    key: { value: 'test-only-key' }, repository: { value: 'example' }, submit: { disabled: false },
    workflow: { files: [{ name: 'reviewed.yml', size: 10, arrayBuffer: async () => new TextEncoder().encode('name: test').buffer }] },
  };
  const state = { textContent: '' };
  const form = { elements: { namedItem: name => controls[name] }, reportValidity: () => valid, addEventListener() {} };
  const context = vm.createContext({
    document: { querySelector: selector => selector === '#github-config' ? form : selector === '#github-state' ? state : {} },
    window: { location: { protocol: 'https:', hostname: trusted ? 'app.gnx' : 'untrusted.example' }, setInterval() {} },
    navigator: {}, Uint8Array, btoa: value => Buffer.from(value, 'binary').toString('base64'),
    fetch: async (url, options) => {
      if (url === './status.json') return { ok: false };
      calls.push({ url, options });
      const step = calls.length;
      return { ok: step !== failure, status: step === failure ? 403 : 200,
        headers: { get: () => scopes },
        json: async () => ({ owner: { login: 'operator' }, name: 'example', private: true, default_branch: 'main' }) };
    },
  });
  vm.runInContext(app, context);
  await vm.runInContext('configureGithub({ preventDefault() {} })', context);
  assert.equal(controls.submit.disabled, false);
  if (trusted && valid) assert.equal(controls.key.value, '');
  for (const { url, options } of calls) {
    assert.ok(url.startsWith('https://api.github.com/'));
    assert.equal(options.redirect, 'error');
    assert.equal(options.credentials, 'omit');
    assert.equal(options.headers.Authorization, 'Bearer test-only-key');
    assert.ok(!url.includes('test-only-key'));
  }
  assert.ok(!state.textContent.includes('test-only-key'));
  return { calls, message: state.textContent };
}
const configured = await githubCase();
assert.deepEqual(configured.calls.map(call => call.options.method), ['GET', 'POST', 'PUT']);
assert.equal(configured.calls[2].url, 'https://api.github.com/repos/operator/example/contents/.github/workflows/reviewed.yml');
assert.equal(JSON.parse(configured.calls[1].options.body).private, true);
assert.equal(Buffer.from(JSON.parse(configured.calls[2].options.body).content, 'base64').toString(), 'name: test');
assert.match(configured.message, /configurado/);
assert.equal((await githubCase({ scopes: 'repo' })).calls.length, 1);
assert.equal((await githubCase({ scopes: '' })).calls.length, 1);
assert.equal((await githubCase({ failure: 1 })).calls.length, 1);
assert.equal((await githubCase({ failure: 2 })).calls.length, 2);
assert.match((await githubCase({ failure: 3 })).message, /Repositorio creado; falta instalar/);
assert.equal((await githubCase({ trusted: false })).calls.length, 0);
assert.equal((await githubCase({ valid: false })).calls.length, 0);
console.log('network/PWA static and in-memory GitHub checks passed; live checks remain operator-gated');
