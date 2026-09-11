"""Explicit live-node checks. --recovery authorizes service restart/failure injection.

Run as Linux root: python3 tests/verify_runtime.py --config /etc/gnx/NAME.toml
Only sanitized observations are printed. No credentials or private keys are read.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import time
import tomllib

parser = argparse.ArgumentParser()
parser.add_argument('--config', required=True)
parser.add_argument('--recovery', action='store_true')
args = parser.parse_args()
config = tomllib.loads(pathlib.Path(args.config).read_text())
instance = config['instance']
assert instance.isalnum() and len(instance) <= 32, 'Use a simple named test instance'
root = pathlib.Path('/var/lib/gnx') / instance
names = ['gnx-' + instance + '-' + cap for cap in ('access', 'dns', 'control', 'compute')]

def run(argv, timeout=180):
    return subprocess.run(argv, capture_output=True, text=True, timeout=timeout)

def command(op):
    result = run(['/usr/local/bin/gnx', op, '--config', args.config])
    report = json.loads(result.stdout)
    assert result.returncode == 0 and report['state'] == 'READY', (op, report['code'])
    return {k: report[k] for k in ('operation', 'state', 'code', 'revision', 'capabilities')}

def fingerprint():
    ca = root / 'control/data/caddy/pki/authorities/local/root.crt'
    return hashlib.sha256(ca.read_bytes()).hexdigest()

def containers():
    return [run(['podman', 'inspect', '--format', '{{.Id}} {{.State.StartedAt}}', n]).stdout.strip() for n in names]

def ready():
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        try:
            return command('status')
        except (AssertionError, json.JSONDecodeError):
            time.sleep(2)
    raise AssertionError('Recovery exceeded 120 seconds')

evidence = {'schema': 1, 'version': run(['/usr/local/bin/gnx', '--version']).stdout.strip(), 'instance': instance}
evidence['doctor'] = command('doctor')
baseline = command('status')
before = containers()
ca = fingerprint()
managed = [root / 'dns/gnx.zone', root / 'dns/Corefile', root / 'control/Caddyfile', root / 'compute/root-ca.pem']
mtimes = [p.stat().st_mtime_ns for p in managed]
for op in ('plan', 'plan', 'apply', 'apply'):
    assert command(op)['revision'] == baseline['revision']
assert containers() == before, 'Unchanged apply restarted a container'
assert [p.stat().st_mtime_ns for p in managed] == mtimes, 'Unchanged apply rewrote runtime assets'
evidence['idempotence'] = 'PASS'
evidence['root_sha256'] = ca
if args.recovery:
    evidence['recovery'] = []
    for name in names:
        assert run(['systemctl', 'restart', name + '.service']).returncode == 0
        assert ready()['revision'] == baseline['revision']
        assert fingerprint() == ca
        evidence['recovery'].append({'service': name, 'restart': 'PASS'})
    assert run(['systemctl', 'kill', '--signal=SIGKILL', '--kill-whom=main', names[0] + '.service']).returncode == 0
    time.sleep(7)
    assert ready()['revision'] == baseline['revision']
    assert fingerprint() == ca
    evidence['unexpected_access_exit'] = 'PASS'
evidence['status'] = command('status')
print(json.dumps(evidence, indent=2))
