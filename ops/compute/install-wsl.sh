#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ]
repo=$(realpath "$1")
state=$(realpath "$2")
runtime=/var/lib/gnx/compute
test -c /dev/kvm
test -c /dev/net/tun
systemctl is-active --quiet gnx-control.service
install -d -m 700 "$runtime" "$runtime/storage" "$runtime/config"
if [ -f "$runtime/root.password" ]; then
    cmp -s "$state/root.password" "$runtime/root.password" || { echo 'FAILED COMPUTE_IDENTITY'; exit 1; }
else
    test ! -f "$runtime/config/config.db"
    install -m 600 "$state/root.password" "$runtime/root.password"
fi
install -d -m 755 /usr/local/lib/gnx/compute /etc/containers/systemd
install -m 755 "$repo/ops/compute/entrypoint.sh" /usr/local/lib/gnx/compute/entrypoint.sh
install -m 644 "$repo/runtime/compute/gnx-compute.container" /etc/containers/systemd/gnx-compute.container
systemctl daemon-reload
systemctl start gnx-compute.service
ready=false
i=0
while [ "$i" -lt 60 ]; do
    if podman exec gnx-compute curl --silent --fail --max-time 3 --cacert /etc/pve/pve-root-ca.pem https://gnx-compute:8006/ -o /dev/null 2>/dev/null; then ready=true; break; fi
    i=$((i + 1)); sleep 3
done
[ "$ready" = true ] || { echo 'FAILED COMPUTE_UPSTREAM_TLS'; exit 1; }
echo 'READY compute-upstream'
