// Format/integrity validation and deterministic packaging, using Node's standard library.
// No Rust, desktop or container build, guest provisioning, release write or host mutation.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const base = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(base, '../..');
const manifest = JSON.parse(readFileSync(join(base, 'manifest.json'), 'utf8'));
const run = (cmd, args, options = {}) => execFileSync(cmd, args, { encoding: 'utf8', ...options });
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const json = (file, value) => writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
const files = (dir) => readdirSync(dir).sort().flatMap((name) => {
  const path = join(dir, name);
  assert(!lstatSync(path).isSymbolicLink(), `Symlink forbidden: ${path}`);
  return lstatSync(path).isDirectory() ? files(path) : [path];
});
const copy = (from, to) => { mkdirSync(dirname(to), { recursive: true }); copyFileSync(from, to); };

assert.equal(manifest.schema, 1);
assert.equal(manifest.name, 'gnx-preview');
assert.match(manifest.version, /^\d+\.\d+\.\d+$/);
assert.equal(manifest.platform, 'linux/amd64');
assert.equal(manifest.desktop.sha256, '4af141d9a8f48cee8cd5389eed5ff5b3651f681f8c11e2039e9bdd67fd03443f');
assert.equal(manifest.desktop.release, 'v0.2.0-261127-4d08ae3');
assert.equal(manifest.desktop.url, `https://github.com/mayas-alas/gnxkernel/releases/download/${manifest.desktop.release}/gnx-desktop.tar.gz`);
assert.equal(manifest.desktop.session, 'debian');
assert.equal(manifest.desktop.directory, '/var/lib/gnx-app/preview/desktop');
assert.equal(manifest.desktop.archive, 'payload/gnx-desktop.tar.gz');
assert.equal(manifest.images.node.version, '26.13.1-bookworm-slim');
assert.equal(manifest.images.memos.version, '26.11.2');
assert.equal(manifest.services.ui.origin, 'https://ui.kernel.gnx');
assert.equal(manifest.services.notas.origin, 'https://notas.gnx');
assert.deepEqual(manifest.services.compute, { origin: 'https://compute.gnx', managed: false });
assert.equal(manifest.volumes.memos.target, '/var/opt/memos');
assert.equal(manifest.volumes.memos.source, '/var/lib/gnx-app/preview/memos');
assert.equal(manifest.volumes.config, '/etc/gnx-app/config/preview');
const units = { node: 'preview-ui.container', memos: 'preview-memos.container' };
for (const [role, unit] of Object.entries(units)) {
  const image = manifest.images[role];
  assert.match(image.reference, new RegExp(`^docker\\.io/${role === 'node' ? 'library/node' : 'neosmemo/memos'}@sha256:[a-f0-9]{64}$`));
  assert.equal(image.archive, `images/${role}.oci.tar`);
  const text = readFileSync(join(base, 'quadlets', unit), 'utf8');
  assert.equal(text.split('\n').filter((line) => line.startsWith('Image=')).join(''), `Image=${image.reference}`);
  assert(text.includes('Pull=never\n') && text.includes('WantedBy=multi-user.target\n'));
  assert(!/PodmanArgs=|AddDevice=|ExecStart=|Network=host/.test(text));
  for (const line of text.split('\n').filter((line) => line.startsWith('Volume='))) {
    assert.match(line, /^Volume=\/(var\/lib\/gnx-app|etc\/gnx-app\/config)\//);
  }
}
const config = JSON.parse(readFileSync(join(base, 'config/memos/instance.json'), 'utf8'));
assert.equal(config.instanceSettings.find((s) => s.name === 'instance/settings/ACCESS').accessSetting.accessMode, 'INSTANCE_ACCESS_MODE_PRIVATE');
assert.equal(config.instanceSettings.find((s) => s.name === 'instance/settings/GENERAL').generalSetting.disallowUserRegistration, true);
const admission = files(join(base, 'quadlets')).concat(files(join(base, 'config')));
assert(admission.length <= 32);
assert(admission.every((file) => readFileSync(file).length <= 16384));
assert(admission.reduce((total, file) => total + readFileSync(file).length, 0) <= 49152);
for (const file of files(base)) {
  assert(!readFileSync(file).includes(13), `CRLF forbidden: ${file}`);
}
const caddy = readFileSync(join(repo, 'config/gateway/Caddyfile'), 'utf8');
assert(caddy.includes('reverse_proxy https://127.0.0.1:8006'));
assert(caddy.includes('tls_server_name compute.gnx') && caddy.includes('pem_file /etc/gnx/tls/root.crt'));
assert(!caddy.includes('tls_insecure_skip_verify'));
for (const name of ['ui.kernel.gnx', 'notas.gnx']) {
  assert(readFileSync(join(repo, 'config/pki/node.ext'), 'utf8').includes(`DNS:${name}`));
  assert(readFileSync(join(repo, 'scripts/tls.run'), 'utf8').includes(name));
  assert(readFileSync(join(repo, 'scripts/configure.run'), 'utf8').includes(name));
  assert(readFileSync(join(repo, 'apps/gnx/src/windows/access.rs'), 'utf8').includes(`"${name}"`));
}
console.log('Preview format, pins, paths, private settings and TLS integration validated.');
if (!process.argv.includes('--package')) process.exit(0);
assert.equal(process.platform, 'linux', 'Packaging requires GNU tar, gzip, curl and skopeo on Linux');
assert.equal(run('git', ['status', '--porcelain', '--untracked-files=no'], { cwd: repo }).trim(), '', 'Commit changes before packaging');
const commit = run('git', ['rev-parse', 'HEAD'], { cwd: repo }).trim();
const epoch = run('git', ['show', '-s', '--format=%ct', 'HEAD'], { cwd: repo }).trim();
const output = join(repo, 'build', `gnx-preview-${manifest.version}-${commit}`);
assert(!existsSync(output), 'Output exists; inspect it before a new packaging attempt');
const bundle = join(output, 'bundle');
mkdirSync(bundle, { recursive: true });
for (const file of files(base)) copy(file, join(bundle, relative(base, file)));
for (const path of ['config/pki/node.ext', 'config/compute/stack-dns.conf', 'scripts/tls.run', 'scripts/configure.run']) {
  copy(join(repo, path), join(bundle, 'integration', path));
}
mkdirSync(join(bundle, 'images'), { recursive: true });
mkdirSync(join(bundle, 'payload'), { recursive: true });
run('curl', ['--fail', '--silent', '--show-error', '--location', '--proto', '=https', '--proto-redir', '=https', '--output', join(bundle, manifest.desktop.archive), manifest.desktop.url]);
assert.equal(sha(readFileSync(join(bundle, manifest.desktop.archive))), manifest.desktop.sha256, 'Desktop SHA-256 mismatch');
const desktopFiles = run('tar', ['-tzf', join(bundle, manifest.desktop.archive)]).split('\n');
assert(desktopFiles.some((name) => /^\.\/server\/index\.mjs$/.test(name)));
assert(desktopFiles.some((name) => /public\/distros\/debian-[a-f0-9]+\.erofs$/.test(name)), 'Published Debian image missing');
assert(desktopFiles.every((name) => !name.startsWith('/') && !name.split('/').includes('..')), 'Unsafe desktop archive path');
const tar = (source, dest) => run('tar', ['--sort=name', `--mtime=@${epoch}`, '--owner=0', '--group=0', '--numeric-owner', '--format=gnu', '-cf', dest, '-C', source, '.']);
const imageConfigs = {};
for (const [role, image] of Object.entries(manifest.images)) {
  const input = join(output, 'inputs', role);
  mkdirSync(dirname(input), { recursive: true });
  run('skopeo', ['copy', '--preserve-digests', `docker://${image.reference}`, `oci:${input}:${role}`]);
  const inspect = JSON.parse(run('skopeo', ['inspect', `oci:${input}:${role}`]));
  assert.equal(`${inspect.Os}/${inspect.Architecture}`, manifest.platform);
  const raw = run('skopeo', ['inspect', '--raw', `oci:${input}:${role}`], { encoding: 'buffer' });
  assert.equal(`sha256:${sha(raw)}`, image.reference.split('@')[1]);
  imageConfigs[role] = JSON.parse(raw).config.digest;
  tar(input, join(bundle, image.archive));
}
json(join(bundle, 'provenance.json'), {
  schema: 1, source: 'https://github.com/mayas-alas/quetzalcoatl', commit,
  source_date_epoch: Number(epoch), version: manifest.version,
  desktop: manifest.desktop, images: manifest.images, image_configs: imageConfigs,
  packaging: 'Existing upstream desktop and OCI manifests; no compilation',
  tools: { skopeo: run('skopeo', ['--version']).trim(), tar: run('tar', ['--version']).split('\n')[0] }
});
const sums = (dir, list) => writeFileSync(join(dir, 'SHA256SUMS'), list.map((file) => `${sha(readFileSync(file))}  ${relative(dir, file).replaceAll('\\', '/')}`).join('\n') + '\n');
sums(bundle, files(bundle));
run('sha256sum', ['--strict', '-c', 'SHA256SUMS'], { cwd: bundle });
const name = `gnx-preview-${manifest.version}.tar`;
tar(bundle, join(output, name));
run('gzip', ['-n', join(output, name)]);
copy(join(bundle, 'manifest.json'), join(output, 'manifest.json'));
copy(join(bundle, 'provenance.json'), join(output, 'provenance.json'));
sums(output, [join(output, `${name}.gz`), join(output, 'manifest.json'), join(output, 'provenance.json')]);
console.log(`Offline bundle and checksums: ${output}`);
