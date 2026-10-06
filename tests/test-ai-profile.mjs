import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = 'payload/node/workloads/ai';
const text = path => readFileSync(path, 'utf8');
const profile = JSON.parse(text(join(root, 'profile.json')));
let checks = 0;
function check(label, test) { test(); checks++; console.log(`PASS ${label}`); }
function files(path) {
  return readdirSync(path).flatMap(name => {
    const child = join(path, name);
    return statSync(child).isDirectory() ? files(child) : [child];
  });
}
function value(unit, key) {
  return unit.split(/\r?\n/).find(line => line.startsWith(`${key}=`))?.slice(key.length + 1);
}

check('external, unprivileged, isolated machine profile', () => {
  assert.equal(profile.kind, 'gnx.machine.profile');
  assert.equal(profile.driver, 'proxmox-lxc');
  assert.equal(profile.name, 'ai');
  assert.equal(profile.unprivileged, true);
  assert.equal(profile.ownership_tag, 'gnx-ai');
  assert.equal(profile.public_ingress, false);
  assert.equal(profile.continuity.scope, 'ai');
  assert.equal(profile.continuity.prefix, 'continuity/ai');
});
check('six real Quadlets, no Python files or .in templates', () => {
  assert.equal(profile.quadlets.length, 6);
  assert.equal(new Set(profile.quadlets).size, 6);
  for (const file of files(root)) assert.ok(!/\.(py|in)$/.test(file), file);
});
let memory = 0;
for (const file of profile.quadlets) {
  check(`${file}: pinned, private, external and bounded`, () => {
    const unit = text(join(root, file));
    assert.match(value(unit, 'Image'), /^localhost\/[a-z0-9/-]+@sha256:[a-f0-9]{64}$/);
    assert.equal(value(unit, 'Pull'), 'never');
    assert.equal(value(unit, 'NoNewPrivileges'), 'true');
    assert.equal(value(unit, 'Requires'), 'workload-mesh-guard.service');
    assert.ok(!/@IMAGE@|GNX_COMPUTE_GATEWAY/.test(unit));
    assert.ok(!/gateway-ca\.key|private-ca\.key|tskey-auth-[A-Za-z0-9_-]+|hf_[A-Za-z0-9]{20,}/.test(unit));
    const limit = /--memory=(\d+)m\b/.exec(value(unit, 'PodmanArgs') ?? '');
    assert.ok(limit, 'memory limit required');
    memory += Number(limit[1]);
    for (const line of unit.split(/\r?\n/).filter(line => line.startsWith('EnvironmentFile='))) {
      assert.ok(line.startsWith('EnvironmentFile=/var/lib/gnx/ai/secrets/'));
    }
  });
}
check('memory leaves room for guest system services', () => assert.ok(memory <= profile.memory_mib - 512));
check('Tailscale has its own state and no copied identity', () => {
  const mesh = text(join(root, 'workload-mesh.container'));
  assert.equal(value(mesh, 'ContainerName'), 'gnx-ai-mesh');
  assert.ok(value(mesh, 'PodmanArgs').includes(`--hostname=${profile.mesh.hostname}`));
  assert.equal(profile.mesh.copy_existing_identity, false);
  assert.ok(mesh.includes(`Volume=${profile.mesh.state_dir}:/var/lib/tailscale:Z`));
  assert.ok(!mesh.includes('Volume=/var/lib/gnx/mesh:'));
});
check('Continuity AI never mounts the business journal', () => {
  const unit = text(join(root, 'continuity.container'));
  assert.equal(value(unit, 'Volume'), '/var/lib/continuity-ai:/data');
  assert.equal(value(unit, 'Environment'), 'CONTINUITY_DATA_DIR=/data');
  assert.ok(unit.includes('Environment=CONTINUITY_SCOPE=ai'));
  assert.ok(unit.includes('Environment=CONTINUITY_S3_PREFIX=continuity/ai'));
  assert.ok(!unit.includes('continuity-business') && !unit.includes('continuity/business'));
});
check('speech is not falsely enabled without inference credentials', () => {
  const unit = text(join(root, 'voice.container'));
  assert.ok(unit.includes('Environment=VOICE_ENABLED=false'));
  assert.ok(!unit.includes('HF_TOKEN='));
});
check('packaging is shell and validates actual Quadlets, without rendering', () => {
  const script = text('installer/scripts/package-ai.run');
  assert.ok(script.startsWith('#!/usr/bin/env bash\n'));
  assert.ok(!/python3?|\.container\.in|@IMAGE@/.test(script));
  assert.ok(script.includes('QUADLET_IMAGE_DRIFT'));
  assert.ok(script.includes('IMAGE_IMPORT_IDENTITY_FAILED'));
});
console.log(`${checks} AI definition contracts passed; NOT live deployment acceptance.`);
