import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
const read = name => readFileSync(new URL(`../payload/node/workloads/private-services/${name}`, import.meta.url), 'utf8');
const nft = read('mesh.nft');
const mesh = read('workload-mesh.container');
const guard = read('workload-mesh-guard.service');
const rustfs = read('rustfs.container');
const install = read('install.run');
const aiInstall = readFileSync(new URL('../payload/node/workloads/ai/install.run', import.meta.url), 'utf8');
const adapter = read('mesh.run');
assert.match(nft, /chain output\s*{\s*type filter hook output priority -150; policy drop;/);
assert.match(nft, /chain forward\s*{\s*type filter hook forward priority -150; policy drop;/);
assert.match(nft, /oifname "eth0" meta skuid 65532 accept/);
const output = nft.split('chain output {')[1].split('chain forward')[0];
assert.doesNotMatch(output, /ct state|policy accept/);
assert.match(mesh, /User=65532:65532/);
assert.match(mesh, /Network=host/);
assert.match(mesh, /Pull=never/);
assert.match(mesh, /DropCapability=all/);
assert.match(mesh, /NoNewPrivileges=true/);
assert.doesNotMatch(mesh, /EnvironmentFile|TS_AUTHKEY|--auth-key/);
assert.match(install, /install -d -m 0700 -o 65532 -g 65532 \/var\/lib\/gnx\/mesh/);
assert.match(aiInstall, /install -d -m 0700 -o 65532 -g 65532 "\$state\/mesh"/);
assert.doesNotMatch(mesh, /Privileged=true/i);
assert.deepEqual([...mesh.matchAll(/^AddCapability=([A-Z0-9_]+)$/gm)].map(match => match[1]), ['NET_ADMIN','NET_RAW']);
assert.match(guard, /^ExecStart=\/usr\/local\/libexec\/gnx-workload-mesh guard$/m);
assert.doesNotMatch(guard, /^ExecStop=/m);
assert.match(rustfs, /Requires=workload-mesh.service workload-mesh-guard.service/);
assert.match(rustfs, /Network=host/);
assert.match(rustfs, /DropCapability=all/);
assert.doesNotMatch(rustfs, /PublishPort=/);
assert.match(rustfs, /RUSTFS_ADDRESS=127.0.0.1:9000/);
assert.match(rustfs, /RUSTFS_CONSOLE_ADDRESS=0.0.0.0:443/);
assert.match(rustfs, /RUSTFS_TLS_PATH=\/certs/);
assert.match(rustfs, /AddCapability=NET_BIND_SERVICE/);
assert.match(rustfs, /tls:\/certs:ro,Z/);
assert.match(adapter, /--cacert \/var\/lib\/gnx\/rustfs\/tls\/ca.crt/);
assert.match(adapter, /https:\/\/rustfs.gnx\//);
assert.doesNotMatch(adapter, /mesh serve/);
assert.doesNotMatch(adapter, /mesh funnel|--https=0/);
assert.match(install, /gnx-workload-mesh https/);
assert.doesNotMatch(install, /apt-get|--pull=always/);
assert.match(install, /OFFLINE_BOOTSTRAP_REQUIRED/);
assert.match(install, /INVALID_ARGUMENT/);
assert.match(install, /"\$\{1:-\}" != --prepare/);
assert.match(install, /networkd-mesh-guard.conf/);
assert.match(read('networkd-mesh-guard.conf'), /Requires=workload-mesh-guard.service/);
assert.ok(install.indexOf('systemctl enable --now workload-mesh-guard.service') < install.indexOf('"${1:-}" != --prepare'));
assert.ok(install.indexOf('gnx-workload-mesh status') < install.indexOf('podman build'));
assert.doesNotMatch(install, /gnx-workload-mesh verify/);
assert.match(install, /dpkg -i/);
assert.match(install, /offline.sha256/);
assert.match(adapter, /--exit-node-allow-lan-access=false/);
assert.match(adapter, /\.Online == true and \.ExitNodeOption == true/);
assert.match(adapter, /length == 1/);
assert.match(adapter, /\.ExitNodeStatus.Online == true/);
assert.match(adapter, /--auth-key=file:/);
assert.match(adapter, /mesh\(\) \{ podman exec "\$mesh_container" tailscale/);
assert.match(adapter, /mesh_root\(\) \{ podman exec --user 0 "\$mesh_container" tailscale/);
const classifierStart = adapter.indexOf('classify_cert_error() {');
const classifierEnd = adapter.indexOf('\ncase "$1" in', classifierStart);
assert.ok(classifierStart >= 0 && classifierEnd > classifierStart, 'missing certificate error classifier');
const classifier = adapter.slice(classifierStart, classifierEnd);
const classifierCases = [
  ['LocalAPI permission denied', 'CERT_ACCESS_DENIED'],
  ['open backend.key: permission denied', 'CERT_FILE_PERMISSION_DENIED'],
  ['HTTPS certificates are not enabled', 'CERT_HTTPS_DISABLED'],
  ['certificate domain not allowed', 'CERT_DOMAIN_DISALLOWED'],
  ['ACME rate limit exceeded', 'CERT_RATE_LIMITED'],
  ['ACME challenge failed', 'CERT_ACME_FAILED'],
  ['network is unreachable', 'CERT_NETWORK_FAILED'],
  ['no such host', 'CERT_DNS_FAILED'],
  ['system time is wrong', 'CERT_CLOCK_INVALID'],
  ['read-only file system', 'CERT_READONLY_FILESYSTEM'],
  ['unknown flag: --cert-file', 'CERT_CLI_UNSUPPORTED'],
  ['unexpected certificate response', 'CERT_UNKNOWN'],
];
const classifierFixture = ['set -euo pipefail', classifier,
  ...classifierCases.map(([input]) => `classify_cert_error ${JSON.stringify(input)}; printf '\\n'`)].join('\n') + '\n';
const classified = process.platform === 'win32'
  ? spawnSync('wsl.exe', ['-d', 'GnX-LinuxLab', '--exec', 'bash', '-s'], { input: classifierFixture, encoding: 'utf8' })
  : spawnSync('bash', ['-s'], { input: classifierFixture, encoding: 'utf8' });
assert.equal(classified.status, 0, classified.stderr || String(classified.error));
assert.deepEqual(classified.stdout.trim().split('\n'), classifierCases.map(([, expected]) => expected));
assert.match(adapter, /cert_output=\$\(mesh_root cert[\s\S]*2>&1\)/);
assert.match(adapter, /MESH_TLS_CERT_REQUIRED/);
assert.match(adapter, /cert_reason=\$\(classify_cert_error/);
const inspector = readFileSync(new URL('./node-runtime/inspect-ai.run', import.meta.url), 'utf8');
for (const category of ['chmod_state_dir','mkdir_cert_dir','read_state_file','write_state_file',
  'socket_create','state_dir_permissions','identity_load','unknown']) {
  assert.match(inspector, new RegExp(`['"]${category}['"]`));
}
assert.match(inspector, /declares_daemon_65532_mode_0700/);
assert.match(inspector, /tailscale_status':mesh_status\(vmid, container, False\)/);
assert.match(nft, /iifname "tailscale0" ip saddr \$gnx_gateway tcp dport 7310 accept/);
assert.doesNotMatch(nft, /iifname "tailscale0" tcp dport 7310 accept/);
assert.match(adapter, /--accept-dns=true/);
assert.doesNotMatch(adapter, /--accept-dns=false/);
assert.doesNotMatch(adapter, /--exit-node=""|--exit-node=auto/);
const lxc = readFileSync(new URL('../payload/node/compute/lxc.run', import.meta.url), 'utf8');
const base = readFileSync(new URL('../payload/node/install.run', import.meta.url), 'utf8');
const host = readFileSync(new URL('../apps/host-agent/src/host_access.rs', import.meta.url), 'utf8');
assert.match(lxc, /address=\/rustfs\.mesh\.gnx\/%s/);
assert.doesNotMatch(lxc, /address=\/rustfs\.gnx\/%s/);
assert.match(lxc, /WORKLOAD_GATEWAY_HTTPS_FAILED/);
assert.match(lxc, /https:\/\/rustfs\.gnx\/rustfs\/console\/[\s\S]*then service=healthy/);
assert.match(base, /address=\/rustfs\.gnx\/%s/);
assert.match(base, /make_leaf gateway-ca rustfs/);
assert.match(base, /if \[\[ "\$mode" == reconcile \]\]; then[\s\S]*generate_ca_material[\s\S]*configure_private_dns/);
assert.match(base, /install -m 0755 "\$payload_dir\/compute\/lxc.run" "\$runtime_dir\/lxc.run"/);
assert.match(base, /s@address=\/rustfs.gnx\/@address=\/rustfs.mesh.gnx\/@/);
const linux = readFileSync(new URL('../apps/host-agent/src/linux_node.rs', import.meta.url), 'utf8');
assert.match(linux, /pub fn reconcile_ingress[\s\S]*"reconcile"/);
const verify = readFileSync(new URL('../payload/node/verify.run', import.meta.url), 'utf8');
assert.match(verify, /check dns_rustfs pass/);
assert.match(verify, /check rustfs_gateway_https pass/);
assert.match(verify, /"\$runtime_dir\/lxc.run" status rustfs/);
const hostConfig = JSON.parse(readFileSync(new URL('../payload/host/access-hosts.json', import.meta.url), 'utf8'));
assert.ok(hostConfig.hosts.includes('rustfs.gnx'));
assert.match(host, /owned_hostnames\(\)/);
assert.match(host, /127\.0\.0\.1 \{name\}/);
assert.match(host, /access-hosts\.json/);
console.log('Workload mesh static contracts passed (not live network acceptance).');
