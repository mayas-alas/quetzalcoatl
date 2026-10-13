import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
const source = (await readFile(new URL('../../payload/node/compute/reconcile.run', import.meta.url), 'utf8')).replace(/\r/g, '');
const recovery = (await readFile(new URL('../../payload/node/services/readiness.run', import.meta.url), 'utf8')).replace(/\r/g, '');
const library = source.split('# Dispatch')[0].replace('source "$source_dir/services/readiness.run"', recovery);
// Exercise the actual lifecycle functions with memory-only OS fakes. No files,
// disposable helpers, credentials, live units or containers are created.
const fake = `
read_receipt() { printf '%s' "$applied"; }
record_receipt() { applied=$1; trace+="ack;"; }
cmp() { [[ "$installed" == 1 ]]; }
install() { trace+="hook;"; }
bash() {
  trace+="fetch;"
  [[ "$pullfail" == 0 ]] || return 1
  cached=1; installed=1; trace+="copy;"
}
podman() {
  case "$1:$2" in
    image:inspect) [[ "$cached" == 1 ]] || return 1; printf 'sha256:new' ;;
    inspect:--format)
      if [[ "$3" == '{{.Image}}' ]]; then printf '%s' "$live"; else printf '%s' "$status"; fi ;;
    exec:gnx-compute)
      [[ "$execfail" == 0 ]] || return 125
      if [[ "$3" == /usr/bin/awk ]]; then [[ "$blocked" == 0 ]]; return; fi
      printf 'pve-manager\\tinstalled\\n'
      if [[ "$fakepkg" == 1 ]]; then printf 'pve-fake-subscription\\tinstalled\\n'; fi ;;
    *) return 125 ;;
  esac
}
sleep() { :; }
systemctl() {
  case "$1" in
    show) printf 'generation-%s' "$generation"; return 0 ;;
    is-active) [[ "$status" == running ]]; return ;;
  esac
  trace+="$1;"
  if [[ "$restartfail" != 0 && "$autorecover" == 0 ]]; then return 1; fi
  generation=$((generation+1)); live=new; status=running; fakepkg=$afterfake
  [[ "$restartfail" == 0 ]]
}
trap 'printf "TRACE=%s RECEIPT=%s LIVE=%s QUADLET=%s\\n" "$trace" "$applied" "$live" "$installed"' EXIT
`;
function run(mode, overrides={}) {
  const state={applied:'old',installed:0,cached:0,live:'old',status:'running',fakepkg:1,
    afterfake:0,blocked:0,execfail:0,pullfail:0,restartfail:0,generation:0,autorecover:0,...overrides};
  const setup=Object.entries(state).map(([k,v])=>`${k}=${v}`).join('\n');
  return spawnSync('bash', ['-c', library+'\n'+fake+'\n'+setup+'\nimage=registry/image@sha256:abc\nfingerprint=new\ntrace=""\n'+(mode==='force'?'apply_compute --restart':mode==='apply'?'apply_compute':'check_compute')], {encoding:'utf8'});
}
let r=run('apply');
assert.equal(r.status,0,r.stderr); assert.match(r.stdout,/fetch;copy;hook;restart;ack;/);
r=run('apply',{applied:'new',installed:1,cached:1,live:'new',fakepkg:0});
assert.equal(r.status,0,r.stderr); assert.doesNotMatch(r.stdout,/restart;/); assert.match(r.stdout,/start;ack;/);
// The cached desired image / already copied Quadlet must not hide an old guest.
r=run('apply',{applied:'new',installed:1,cached:1});
assert.equal(r.status,0,r.stderr); assert.match(r.stdout,/restart;ack;/);
r=run('apply',{pullfail:1});
assert.notEqual(r.status,0); assert.match(r.stderr,/COMPUTE_PAYLOAD_INSTALL_FAILED/);
assert.doesNotMatch(r.stdout,/copy;|restart;|ack;/); assert.match(r.stdout,/RECEIPT=old LIVE=old QUADLET=0/);
r=run('apply',{restartfail:1});
assert.notEqual(r.status,0); assert.doesNotMatch(r.stdout,/ack;/); assert.match(r.stdout,/RECEIPT=old LIVE=old QUADLET=1/);
// A scheduled systemd recovery is observed; the lifecycle does not restart twice.
r=run('apply',{restartfail:1,autorecover:1});
assert.equal(r.status,0,r.stderr); assert.match(r.stdout,/restart;ack;/);
assert.equal((r.stdout.match(/restart;/g) ?? []).length,1);
// Retrying a failed activation still recreates despite identical installed files.
r=run('apply',{installed:1,cached:1});
assert.equal(r.status,0,r.stderr); assert.match(r.stdout,/restart;ack;/);
for (const [overrides,code] of [
  [{applied:'old'},'COMPUTE_UPDATE_PENDING'],
  [{live:'old'},'COMPUTE_IMAGE_MISMATCH'],
  [{fakepkg:1},'COMPUTE_FAKE_SUBSCRIPTION_PRESENT'],
  [{execfail:1},'COMPUTE_FAKE_SUBSCRIPTION_PRESENT'],
  [{blocked:1},'COMPUTE_LICENSE_HOSTS_BLOCKED'],
]) {
  r=run('check',{applied:'new',installed:1,cached:1,live:'new',fakepkg:0,...overrides});
  assert.notEqual(r.status,0); assert.match(r.stderr,new RegExp(code)); assert.doesNotMatch(r.stdout,/ack;|restart;|copy;/);
}
r=run('apply',{afterfake:1}); assert.notEqual(r.status,0); assert.doesNotMatch(r.stdout,/ack;/);
r=run('apply',{blocked:1}); assert.notEqual(r.status,0); assert.doesNotMatch(r.stdout,/ack;/);
r=run('force',{applied:'new',installed:1,cached:1,live:'new',fakepkg:0});
assert.equal(r.status,0,r.stderr); assert.match(r.stdout,/restart;ack;/);
const hostsAwk = source.match(/podman exec gnx-compute \/usr\/bin\/awk '([\s\S]*?)' \/etc\/hosts/)[1];
for (const [hosts,blocked] of [
  ['127.0.0.1 localhost # shop.maurer-it.com\n# ::1 shop.maurer-it.com\n',false],
  ['127.0.0.1 localhost shop.maurer-it.com\n',true],
  ['::1 shop.maurer-it.com\n',true],
]) {
  r=spawnSync('bash',['-c',`awk '${hostsAwk}'`],{input:hosts,encoding:'utf8'});
  assert.equal(r.status,blocked?1:0,r.stderr);
}
console.log('Compute upgrade behavioral checks passed (14 lifecycle + 3 hosts-parser memory-only cases).');
