#!/bin/bash
set -euo pipefail
umask 077
test "$(id -u)" = 0
repo=$(realpath "$1")
source_state=$(realpath "$2")
state=/var/lib/gnx/compute
control=/var/lib/gnx/control
test -c /dev/kvm && test -c /dev/fuse && test -c /dev/net/tun
systemctl is-active --quiet gnx-control.service
install -d -m 700 "$state" "$state/storage" "$state/config" "$control/sites"
if test -f "$state/root.password"; then
    cmp -s "$source_state/root.password" "$state/root.password" || { echo 'FAILED COMPUTE_IDENTITY'; exit 1; }
else
    test ! -f "$state/config/config.db"
    install -m 600 "$source_state/root.password" "$state/root.password"
fi
install -d -m 755 /usr/local/lib/gnx/compute
install -m 755 "$repo/ops/compute/entrypoint.sh" /usr/local/lib/gnx/compute/entrypoint.sh
install -m 644 "$repo/runtime/compute/gnx-compute.container" /etc/containers/systemd/gnx-compute.container
systemctl daemon-reload
systemctl start gnx-compute.service
ready=false
for attempt in {1..60}; do
    if podman exec gnx-compute curl --silent --fail --max-time 3 --cacert /etc/pve/pve-root-ca.pem \
        https://gnx-compute:8006/ -o /dev/null 2>/dev/null; then ready=true; break; fi
    sleep 3
done
$ready || { echo 'FAILED COMPUTE_UPSTREAM_TLS'; exit 1; }
podman cp gnx-compute:/etc/pve/pve-root-ca.pem "$control/tls/compute-ca.crt"
chmod 644 "$control/tls/compute-ca.crt"
exec 9>/run/gnx-control-maintenance.lock
flock -x 9
install -m 600 "$repo/runtime/control/tls.cnf" "$control/tls.cnf"
install -m 755 "$repo/ops/control/refresh-identity.sh" /usr/local/lib/gnx/control/refresh-identity.sh
flock -u 9
/usr/local/lib/gnx/control/refresh-identity.sh --no-restart
entry_image=$(sed -n 's/^Image=//p' "$repo/runtime/control/gnx-entry.container")
podman run --rm --network none --entrypoint caddy \
    -v "$repo/runtime/control/Caddyfile:/etc/caddy/Caddyfile:ro" \
    -v "$control/tls:/etc/gnx/tls:ro" \
    -v "$repo/runtime/compute:/etc/gnx/sites:ro" \
    "$entry_image" validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null 2>&1
flock -x 9
install -m 600 "$repo/runtime/control/Caddyfile" "$control/Caddyfile"
install -m 600 "$repo/runtime/compute/compute.caddy" "$control/sites/compute.caddy"
install -m 644 "$repo/runtime/control/gnx-entry.container" /etc/containers/systemd/gnx-entry.container
systemctl daemon-reload
systemctl restart gnx-entry.service
systemctl is-active --quiet gnx-entry.service gnx-control.service gnx-compute.service
echo 'READY compute-upstream-and-entry'
