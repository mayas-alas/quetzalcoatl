import array
import fcntl
import os
import stat

for path in ("/dev/kvm", "/dev/net/tun", "/dev/fuse"):
    mode = os.stat(path).st_mode
    if not stat.S_ISCHR(mode):
        raise SystemExit(f"{path} is not a character device")

kvm = os.open("/dev/kvm", os.O_RDWR | os.O_CLOEXEC)
try:
    api = fcntl.ioctl(kvm, 0xAE00, 0)
finally:
    os.close(kvm)
if api != 12:
    raise SystemExit(f"KVM API version is {api}, expected 12")

tun = os.open("/dev/net/tun", os.O_RDWR | os.O_CLOEXEC)
try:
    features = array.array("I", [0])
    fcntl.ioctl(tun, 0x800454CF, features, True)
finally:
    os.close(tun)

fuse = os.open("/dev/fuse", os.O_RDWR | os.O_CLOEXEC)
os.close(fuse)
print("KVM_API_VERSION=12;TUN=ready;FUSE=ready")
