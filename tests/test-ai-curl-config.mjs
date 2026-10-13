// Real curl configuration semantics; loopback fixture, not live AI acceptance.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import http from 'node:http';
import { spawn } from 'node:child_process';
const source = await readFile(new URL('../payload/node/workloads/ai/enroll.run', import.meta.url), 'utf8');
const functions = source.slice(source.indexOf('curl_api() {'), source.indexOf('read_pat() {'));
const requests = [];
const server = http.createServer((req, res) => {
  requests.push(req.headers);
  res.end('[]');
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
try {
  const url = `http://127.0.0.1:${server.address().port}`;
  const script = `set -euo pipefail\nfail(){ exit 1; }\nNETBIRD_AUTH_SCHEME=Token\ntmpdir=$(mktemp -d ./ai-curl-fixture.XXXXXX)\ntrap 'rm -rf "$tmpdir"' EXIT\n${functions}\ncurl_api GET ${url} token public-fixture\ncurl_status GET ${url} api public-fixture\n[[ "$REPLY" == 200 ]]\nNETBIRD_AUTH_SCHEME=Bearer\ncurl_api GET ${url} token public-fixture\n`;
  const child = spawn('bash', ['-s'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let diagnostic = '';
  child.stdout.resume();
  child.stderr.on('data', chunk => { diagnostic += chunk; });
  child.stdin.end(script);
  const exit = await new Promise(resolve => child.on('close', resolve));
  assert.equal(exit, 0, diagnostic);
  assert.equal(requests[0].authorization, 'Token public-fixture');
  assert.equal(requests[1]['x-api-key'], 'public-fixture');
  assert.equal(requests[2].authorization, 'Bearer public-fixture');
  console.log('AI curl stdin-config headers passed (real curl, local fixture only).');
} finally { await new Promise(resolve => server.close(resolve)); }
