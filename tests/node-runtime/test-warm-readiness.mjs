import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
const script=(await readFile(new URL('../../payload/node/wait-ready.run',import.meta.url),'utf8')).replace(/\r/g,'');
for (const mode of ['ready','warm','timeout']) {
  const fake=`
SECONDS=0
timeout() {
  [[ "$1" == --kill-after=3 && "$2" == 60 && "$3" == bash ]] || return 64
  shift 3; bash "$@"
}
bash() {
  if [[ '${mode}' == ready || ( '${mode}' == warm && $SECONDS -ge 2 ) ]]; then
    printf '{"ready":true}\\n'; return 0
  fi
  printf '{"ready":false}\\n'; return 1
}
sleep() { if [[ '${mode}' == timeout ]]; then SECONDS=$((SECONDS+301)); else SECONDS=$((SECONDS+2)); fi; }
`;
  const r=spawnSync('bash',['-c',fake+script],{encoding:'utf8'});
  assert.equal(r.status,mode==='timeout'?1:0,r.stderr);
  if(mode==='timeout') { assert.match(r.stderr,/VERIFY_TIMEOUT/); assert.match(r.stdout,/"ready":false/); }
  else { assert.equal(r.stdout,'{"ready":true}\n'); assert.doesNotMatch(r.stdout,/false/); }
}
assert.doesNotMatch(script,/podman pull|systemctl|tailscale up|chpasswd/);
console.log('Warm readiness checks passed (3 memory-only cases; no lifecycle mutations).');
