"""Local integration: no credentials leave the Linux process or protected runtime state."""
import json, secrets, struct, subprocess, sys
intent='''schema = 1
instance = "poc031"
node = "compute"
[network]
subnet = "10.93.0.0/24"
'''.encode()
password=secrets.token_urlsafe(36).encode()
frame=b'GNX1'+bytes([2,1,0,0])+struct.pack('<II',len(intent),len(password))+intent+password
result=subprocess.run([sys.argv[1],'apply','--broker'],input=frame,capture_output=True,timeout=600)
report=json.loads(result.stdout)
assert password not in result.stdout and password not in result.stderr
assert report['operation']=='apply'
print(json.dumps({'exit':result.returncode,'report':report}))
