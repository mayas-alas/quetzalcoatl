"""Read-only probes against an explicit GNX lab instance; exports no secrets."""
import json, subprocess, sys
instance, ip = sys.argv[1:]
assert instance.isalnum()
pid = subprocess.check_output(['podman','inspect','--format','{{.State.Pid}}',f'gnx-{instance}-access'], text=True).strip()
ca = f'/var/lib/gnx/{instance}/control/data/caddy/pki/authorities/local/root.crt'
for name in ['compute.gnx','proxmox.gnx']:
    for path in ['/', '/api2/json/version']:
        out = subprocess.run(['nsenter','--target',pid,'--net','--','curl','--silent','--show-error','--noproxy','*','--max-time','10','--cacert',ca,'--resolve',f'{name}:443:{ip}','-w','\n%{http_code}',f'https://{name}{path}'], capture_output=True, text=True, timeout=15)
        print(json.dumps({'name':name,'path':path,'exit':out.returncode,'result':out.stdout[-300:],'error':out.stderr}))
