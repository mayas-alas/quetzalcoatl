#!/bin/sh
# Build a clean service-owned WSL rootfs, never export a user's live distribution.
set -eu
cd "$(dirname "$0")/../.."
expected=$(awk '$2 == "*ubuntu-noble-wsl-amd64-24.04lts.rootfs.tar.gz" || $2 == "ubuntu-noble-wsl-amd64-24.04lts.rootfs.tar.gz" {print $1}' dist/ubuntu-SHA256SUMS)
test "${#expected}" = 64
printf '%s  %s\n' "$expected" dist/ubuntu-rootfs.tar.gz | sha256sum -c -
stage=$(mktemp -d /tmp/gnx-rootfs.XXXXXX)
cleanup() {
 for path in proc sys dev; do mountpoint -q "$stage/$path" && umount "$stage/$path" || true; done
 # Keep the isolated build tree available for diagnosis. It contains no enrollment or identity.
}
trap cleanup EXIT
tar -xzf dist/ubuntu-rootfs.tar.gz -C "$stage"
chmod 755 "$stage"
mount -t proc proc "$stage/proc"
mount --bind /sys "$stage/sys"
mount --bind /dev "$stage/dev"
rm -f "$stage/etc/resolv.conf"
cp /etc/resolv.conf "$stage/etc/resolv.conf"
printf '#!/bin/sh\nexit 101\n' > "$stage/usr/sbin/policy-rc.d"
chmod 755 "$stage/usr/sbin/policy-rc.d"
chroot "$stage" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get update -qq
chroot "$stage" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq podman systemd systemd-sysv curl openssl dnsutils util-linux ca-certificates uidmap
chroot "$stage" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get upgrade -y -qq
chroot "$stage" apt-get clean
rm -f "$stage/usr/sbin/policy-rc.d" "$stage/var/lib/dbus/machine-id"
: > "$stage/etc/machine-id"
printf '[boot]\nsystemd=true\n[automount]\nenabled=false\n[interop]\nenabled=false\nappendWindowsPath=false\n' > "$stage/etc/wsl.conf"
cleanup
tar -C "$stage" -czf dist/gnx-wsl-rootfs.tar.gz .
sha256sum dist/gnx-wsl-rootfs.tar.gz
