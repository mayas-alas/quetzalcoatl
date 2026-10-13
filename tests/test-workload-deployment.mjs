// Contract regressions found through installed-product deployment, not new runners.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const installerBuild = read('installer/scripts/build.ps1');
const podmanBuild = installerBuild.slice(installerBuild.indexOf('if ($BuildPodmanInput)'), installerBuild.indexOf('$hostPayloadSource'));
const podmanInput = JSON.parse(read('installer/inputs/podman.json'));
assert.match(installerBuild, /BuildPodmanInput/);
assert.equal(podmanInput.version, '6.1.3');
for (const helper of ['netavark', 'aardvark-dns']) {
  assert.equal(podmanInput.helpers[helper].version, '2.1.0');
  assert.match(podmanInput.helpers[helper].sha256, /^[a-f0-9]{64}$/);
  assert.ok(podmanBuild.includes(helper));
}
assert.match(podmanBuild, /build\\podman\\inputs/);
assert.match(podmanBuild, /Get-FileHash[^\n]+SHA256/);
assert.match(podmanBuild, /--distribution', 'GnX-LinuxLab'/);
assert.match(podmanBuild, /--exec[\s\S]*tar/);
assert.doesNotMatch(podmanBuild, /gnxnodesvc|Containerfile|bash -c|sh -c|podman-remote/);
assert.match(podmanBuild, /\$buildRoot = '\/var\/lib\/gnx-build\/podman'/);
for (const directory of ['source', 'toolchain', 'cache', 'stage']) assert.ok(podmanBuild.includes(`$buildRoot/${directory}`));
for (const setting of ['GOTOOLCHAIN=local', 'GOPROXY=off', 'GOSUMDB=off', 'GOCACHE=', 'GOMAXPROCS=2', 'GOFLAGS=-mod=vendor', 'GO=', 'GIT_COMMIT=', 'BUILD_INFO=', 'BUILDTAGS=']) assert.ok(podmanBuild.includes(setting));
assert.equal(podmanInput.build.prefix, '/usr');
assert.ok(podmanBuild.includes('PREFIX=$($podman.build.prefix)'));
assert.match(podmanBuild, /'make', '--directory', \$source, '-j2', 'podman', 'rootlessport', 'quadlet'/);
assert.match(podmanBuild, /'install\.bin', 'install\.systemd'/);
assert.match(podmanBuild, /--sort=name[\s\S]*--mtime=@0[\s\S]*--numeric-owner/);
assert.match(podmanBuild, /build-manifest\.json/);
for (const output of ['nativepodman', 'rootlessport', 'quadlet', 'archive']) assert.match(podmanBuild, new RegExp(output));
assert.match(podmanBuild, /runtime_dependency_bundle = 'not complete'/);
assert.doesNotMatch(podmanBuild, /cargo build|dotnet build/);
assert.match(installerBuild, /cargo build --workspace --release --locked/);
assert.ok(podmanBuild.indexOf('return') < installerBuild.indexOf('cargo build'), 'Podman input mode must return before MSI build');
const host = read('apps/host-agent/src/main.rs');
assert.match(host, /from_millis\(250\)/, 'keepalive must yield the managed-profile mutex');
const reconcile = read('payload/node/compute/reconcile-access.run');
assert.doesNotMatch(reconcile, /lxc\.run" start ai/);
assert.match(reconcile, /deployment\.enabled==true/);
assert.match(reconcile, /lxc\.run" start continuity/);
const provision = read('apps/host-agent/src/provisioning.rs');
const snapshot = provision.slice(provision.indexOf('if req.operation == Operation::GetProgress'), provision.indexOf('if matches!(req.operation, Operation::ImportWorkload'));
assert.doesNotMatch(snapshot, /linux_node::/, 'GetProgress must never wait for the WSL identity mutex');
const state = read('apps/host-agent/src/state.rs');
const save = state.slice(state.indexOf('pub fn save('), state.indexOf('pub fn refresh_app_status('));
assert.doesNotMatch(save, /linux_node::(?:runtime_checks|workload_statuses|publish_app_status)/,
  'checkpoint acceptance/finalization must never wait on Linux observation');
assert.match(state, /latest\.sequence != progress\.sequence/);
assert.match(state, /APP_STATUS_FILE\}\.\{\}\.tmp/);
const lxc = read('payload/node/compute/lxc.run');
const profileInstaller = read('payload/node/workloads/ai/install.run');
assert.match(profileInstaller, /runtime\.input_manifest/);
assert.match(profileInstaller, /secrets\.references/);
assert.match(profileInstaller, /PROFILE_SECRET_REQUIRED/);
assert.match(profileInstaller, /Pull=never/);
assert.doesNotMatch(profileInstaller, /voice_api_token|oryx\.env|omniroute\.env|HF_TOKEN/);
assert.doesNotMatch(lxc, /mesh=\$\([^\n]*\|\| printf false/, 'a failed observer must not emit two JSON boolean values');
assert.match(lxc, /profile_exec \/usr\/local\/libexec\/gnx-workload-mesh status[\s\S]*then mesh=true/);
assert.match(lxc, /\[\[ "\$workload" == rustfs \|\| "\$workload" == ai \|\| "\$workload" == continuity \]\]/);
assert.match(lxc, /profile_load\(\)/);
assert.match(lxc, /CONTINUITY_SOURCE_NOT_READY/);
assert.match(lxc, /S3_COMMIT_RESTORE_FAILED/);
assert.match(lxc, /systemctl mask continuity\.service/);
const rollbackSource = lxc.slice(lxc.indexOf('continuity_rollback() {'), lxc.indexOf('continuity_migrate() {'));
const rollbackFixture = `set -euo pipefail
profile_vmid=102
pve() {
  case "$*" in
    *'pct status'*) printf 'status: running\\n' ;;
    *'systemctl start continuity.service'*) source_started=1 ;;
    *) return 0 ;;
  esac
}
profile_exec() {
  case "$*" in
    *'systemctl show'*) [[ "$mode" != unknown ]] || return 1; printf '%s\\n' "$mode" ;;
    *) return 1 ;;
  esac
}
${rollbackSource}
for mode in unknown active inactive failed; do source_started=0; continuity_rollback 100; printf '%s:%s\\n' "$mode" "$source_started"; done
`;
const rollbackTest = process.platform === 'win32'
  ? spawnSync('wsl.exe', ['-d', 'GnX-LinuxLab', '--exec', 'bash', '-s'], { input: rollbackFixture, encoding: 'utf8' })
  : spawnSync('bash', ['-s'], { input: rollbackFixture, encoding: 'utf8' });
assert.equal(rollbackTest.status, 0, rollbackTest.stderr);
assert.deepEqual(rollbackTest.stdout.trim().split('\n'), ['unknown:0', 'active:0', 'inactive:1', 'failed:1']);
assert.match(lxc, /AI_RETIRED_STOPPED/);
assert.doesNotMatch(lxc, /for app in voice|shared_enroll\(\)|ai_mesh_join\(\)/, 'application-specific AI lifecycle must not remain');
assert.doesNotMatch(lxc, /systemctl (?:start|enable) [^\n]*\bai(?:\.service)?\b|pct [^\n]*\bstart [^\n]*\bai\b/, 'retired AI must never be auto-started');
const hostnames = JSON.parse(read('payload/host/access-hosts.json')).hosts;
const profile = JSON.parse(read('payload/node/workloads/ai/profile.json'));
for (const url of Object.values(profile.origins)) assert.ok(hostnames.includes(new URL(url).hostname));
assert.ok(hostnames.includes(profile.custom_domain));
assert.match(read('payload/node/install.run'), /\.cluster_domain, \.custom_domain/);
assert.match(read('payload/node/gateway/routes.conf'), /https:\/\/ai\.gnx \{/);
const routes = read('payload/node/gateway/routes.conf');
assert.match(routes, /https:\/\/continuity\.gnx \{\s+import gnx_service_auth 4185 gnx_business_backend gateway/);
assert.match(routes, /http:\/\/continuity\.mesh\.gnx:7310/);
assert.match(read('payload/node/compute/reconcile-access.run'), /lxc\.run" auth rustfs/);
assert.ok(hostnames.includes('continuity.gnx') && hostnames.includes('continuty.gnx'));
const businessOverride = read('payload/node/workloads/private-services/continuity.container.d/access.conf');
assert.match(businessOverride, /CONTINUITY_LISTEN=0\.0\.0\.0:7310/);
assert.doesNotMatch(businessOverride, /Image=|Volume=|SCOPE=|PREFIX=|EnvironmentFile=/, 'access must not replace the business writer, image or storage');
const dedicatedJournal = read('payload/node/workloads/continuity/continuity.container');
assert.match(dedicatedJournal, /^Environment=CONTINUITY_SCOPE=business$/m);
assert.match(dedicatedJournal, /^Environment=CONTINUITY_S3_PREFIX=continuity\/business$/m);
assert.match(dedicatedJournal, /^Environment=CONTINUITY_ACK=remote$/m);
assert.match(dedicatedJournal, /^Volume=\/var\/lib\/continuity-business:\/data$/m);
assert.doesNotMatch(dedicatedJournal, /RUSTFS_SECRET_KEY|rustfs\.env|\/var\/lib\/continuity-ai/);
for (const file of ['continuity.container', 'netbird-proxy.container']) {
  const unit = read(`payload/node/workloads/continuity/${file}`);
  assert.match(unit, /^Image=\S+@sha256:[a-f0-9]{64}$/m);
  assert.match(unit, /^Pull=never$/m);
  assert.match(unit, /^Requires=workload-mesh-guard\.service$/m);
  assert.match(unit, /^DropCapability=all$/m);
  assert.match(unit, /^NoNewPrivileges=true$/m);
  assert.match(unit, /^WantedBy=multi-user\.target$/m);
}
const dedicatedProxy = read('payload/node/workloads/continuity/netbird-proxy.container');
assert.match(dedicatedProxy, /^User=1000:1000$/m);
assert.match(dedicatedProxy, /^Environment=NB_PROXY_PRIVATE=true$/m);
assert.match(dedicatedProxy, /^Environment=NB_PROXY_ACME_CERTIFICATES=false$/m);
assert.match(dedicatedProxy, /^EnvironmentFile=\/var\/lib\/gnx\/continuity\/secrets\/proxy\.env$/m);
assert.doesNotMatch(dedicatedProxy, /NB_PROXY_TOKEN=|NB_PROXY_ALLOW_INSECURE=true/);
const businessAuth = read('payload/node/auth/business.cfg');
assert.match(businessAuth, /redirect_url = "https:\/\/continuity\.gnx\/oauth2\/callback"/);
assert.match(businessAuth, /cookie_name = "__Host-GnX-Business-Session"/);
assert.match(businessAuth, /allowed_groups = \["gnx-app-users"\]/);
assert.match(businessAuth, /code_challenge_method = "S256"/);
assert.doesNotMatch(businessAuth, /pass_(?:access_token|authorization_header|user_headers) = true/);
// Execute the actual observer with in-memory commands. No guests, state writes
// or scratch runners; WSL supplies the same Bash/JQ used by the installed node.
const observer = lxc.slice(lxc.indexOf('profile_health() {'), lxc.indexOf('profile_provision() {')).replace(/\r/g, '');
const fixture = `set -euo pipefail
workload=ai
profile_dir="$PWD/payload/node/workloads/ai"
profile_identity() { profile_dir="$PWD/payload/node/workloads/ai"; profile_vmid=101; }
pve() { printf 'status: running\\n'; }
profile_exec() {
  case "$1" in
    systemctl|test|openssl) return 0 ;;
    /usr/local/libexec/gnx-workload-mesh)
      case "$mode" in
        true) printf '{"mesh":true}' ;;
        false) printf '{"mesh":false}' ;;
        invalid) printf 'not-json' ;;
        failed) printf '{"mesh":false}'; return 1 ;;
      esac ;;
    *) return 1 ;;
  esac
}
${observer}
for mode in true false invalid failed; do profile_status; done
`;
const executed = process.platform === 'win32'
  ? spawnSync('wsl.exe', ['-d', 'GnX-LinuxLab', '--exec', 'bash', '-s'], { input: fixture, encoding: 'utf8', timeout: 30000 })
  : spawnSync('bash', ['-s'], { input: fixture, encoding: 'utf8', timeout: 30000 });
assert.equal(executed.status, 0, executed.stderr || String(executed.error));
const observed = executed.stdout.trim().split('\n').map(line => JSON.parse(line));
assert.equal(observed.length, 4);
assert.equal(observed[0].mesh, true);
assert.equal(observed[0].service, 'healthy');
for (const result of observed.slice(1)) {
  assert.equal(result.mesh, false);
  assert.equal(result.service, 'attention');
}
const inspector = read('tests/node-runtime/inspect-ai.run').split("python3 - <<'PY'\n")[1]?.replace(/\r/g, '').replace(/\nPY\s*$/, '').replace('__GNX_CADDY_SOURCE__', '"public-source-fixture"');
assert.ok(inspector, 'missing fixed inspector program');
const syntax = process.platform === 'win32'
  ? spawnSync('wsl.exe', ['-d', 'GnX-LinuxLab', '--exec', 'python3', '-c', 'import ast,sys; ast.parse(sys.stdin.read())'], { input: inspector, encoding: 'utf8' })
  : spawnSync('python3', ['-c', 'import ast,sys; ast.parse(sys.stdin.read())'], { input: inspector, encoding: 'utf8' });
assert.equal(syntax.status, 0, syntax.stderr || String(syntax.error));
const deployScript = read('payload/node/workloads/deploy.run');
assert.match(deployScript, /standard_path = builtin \/ 'profile\.json'/);
assert.match(deployScript, /CONTINUITY_MIGRATION_RECEIPT_REQUIRED|migration-receipt\.json/);
assert.match(deployScript, /runtime_manifest/);
const embeddedDeploy = deployScript.slice(deployScript.indexOf("python3 - \"$source\""), deployScript.indexOf("\nPY\n", deployScript.indexOf("python3 - \"$source\""))).split('\n').slice(1).join('\n');
assert.ok(embeddedDeploy, 'missing deploy admission program');
const deploySyntax = process.platform === 'win32'
  ? spawnSync('wsl.exe', ['-d', 'GnX-LinuxLab', '--exec', 'python3', '-c', 'import ast,sys; ast.parse(sys.stdin.read())'], { input: embeddedDeploy, encoding: 'utf8' })
  : spawnSync('python3', ['-c', 'import ast,sys; ast.parse(sys.stdin.read())'], { input: embeddedDeploy, encoding: 'utf8' });
assert.equal(deploySyntax.status, 0, deploySyntax.stderr || String(deploySyntax.error));
assert.match(lxc, /if \[\[ "\$action" == status \]\]; then flock -n 8 \|\| workload_fail WORKLOAD_OBSERVATION_BUSY; else flock 8; fi/);
const node = read('apps/host-agent/src/linux_node.rs');
assert.doesNotMatch(node, /\.arg\("--shutdown"\)/, 'runtime stop must target only its owned distro');
assert.match(node, /"--terminate"\.into\(\), DISTRO_NAME\.into\(\)/);
assert.match(node, /"--kill-after=3", "30", "\/bin\/bash", "\/opt\/gnx\/payload\/node\/verify\.run"/);
const deploy = node.slice(node.indexOf('pub fn deploy_bundle('), node.indexOf('#[cfg(not(windows))]', node.indexOf('pub fn deploy_bundle(')));
assert.match(deploy, /run_wsl_mode\(/);
assert.match(deploy, /"--exec"/);
assert.match(deploy, /"\/opt\/gnx\/payload\/node\/workloads\/deploy\.run"/);
assert.doesNotMatch(deploy, /"-c"|\$@/, 'typed deployment argv must not pass through another shell');
const admission = read('payload/host/workload-bundle.ps1');
assert.match(admission, /\[IO\.File\]::Replace\([^\n]+\[NullString\]::Value\)/);
assert.match(admission, /EnumerateFileSystemInfos\(\)/);
const folder = admission.slice(admission.indexOf('function Get-SourceFolderEntries'), admission.indexOf('function Test-ZipSymlink'));
assert.doesNotMatch(folder, /Get-ChildItem[^\n]*-Recurse/);
assert.match(folder, /\$count -gt \[int64\]\$script:Policy\.limits\.max_entries/);
console.log('PASS: installed-deployment regressions: fast checkpoints, nonblocking observation, typed WSL argv, idempotent receipts and bounded flat-folder enumeration.');
