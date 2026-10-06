import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const read = p => readFileSync(new URL(`../${p}`, import.meta.url), 'utf8');
const routes = read('payload/node/gateway/routes.conf').split('# RustFS retains')[0];
assert.match(routes, /reverse_proxy 127\.0\.0\.1:4180/);
assert.match(routes, /method POST\s+header Origin https:\/\/app.gnx/);
assert.match(routes, /rewrite \* \/oauth2\/sign_out\?rd=https%3A%2F%2Fapp.gnx%2Fsigned-out/);
assert.match(routes, /@internal path \/logout \/oauth2\/sign_out/);
assert.match(routes, /Clear-Site-Data/);
assert.match(routes, /Cache-Control "no-store, private"/);
assert.doesNotMatch(routes, /forward_auth|copy_headers|id_token_hint|token=/);
const config = read('payload/node/auth/oauth2-proxy.cfg');
for (const text of ['allowed_groups = ["gnx-app-users"]', 'code_challenge_method = "S256"', 'insecure_oidc_skip_nonce = false',
  'cookie_name = "__Host-GnX-Session"', 'cookie_secure = true', 'cookie_httponly = true', 'cookie_samesite = "lax"',
  'oidc_email_claim = "sub"', 'session_store_type = "redis"', 'pass_access_token = false', 'pass_authorization_header = false', 'request_logging = false']) assert.ok(config.includes(text));
assert.doesNotMatch(config, /skip_auth|insecure.*= true|cookie_domains|client_secret =/);
for (const unit of ['app-auth', 'app-sessions']) {
  const text = read(`payload/node/services/${unit}.container`);
  assert.match(text, /^Image=.*@sha256:[0-9a-f]{64}$/m);
  for (const option of ['Pull=never', 'User=10001:10001', 'DropCapability=all', 'ReadOnly=true', 'NoNewPrivileges=true']) assert.ok(text.includes(option));
  assert.doesNotMatch(text, /PublishPort|EnvironmentFile|Environment=.*(?:secret|token|password)/i);
}
assert.doesNotMatch(read('payload/node/services/app-auth.container'), /^AddHost=/m, 'Podman 4.9 must generate the unit');
const app = read('apps/web-app/app.js');
assert.match(app, /getRegistrations/);
assert.doesNotMatch(app, /serviceWorker\.register\(/, 'retirement worker must not create a reload loop');
assert.match(app, /if \(!sessionEnded\) renderStatus/);
assert.match(app, /event.persisted.*location.reload/);
// Exercise retirement: delete only GnX caches, unregister, no private offline fetch.
const worker = read('apps/web-app/sw.js');
const listeners = {};
const trace = [];
const ctx = vm.createContext({
  self: { addEventListener: (name, cb) => { listeners[name] = cb; }, skipWaiting: async () => trace.push('skip'),
    registration: { unregister: async () => trace.push('unregister') },
    clients: { claim: async () => trace.push('claim'), matchAll: async () => [{ url: 'https://app.gnx/', navigate: async () => trace.push('navigate') }] } },
  caches: { keys: async () => ['gnx-app-v8', 'unrelated-cache'], delete: async key => trace.push(key) },
});
vm.runInContext(worker, ctx);
let done;
listeners.activate({ waitUntil: promise => { done = promise; } });
await done;
assert.deepEqual(trace, ['gnx-app-v8', 'claim', 'unregister', 'navigate']);
assert.equal(listeners.fetch, undefined);
assert.doesNotMatch(worker, /respondWith|cache\.addAll|caches\.match/);
assert.match(read('apps/web-app/index.html'), /Máquina de servicios privados/);
console.log('PWA auth boundary, logout routing, Podman 4.9 and retirement-worker behavior passed; not personal login evidence.');
