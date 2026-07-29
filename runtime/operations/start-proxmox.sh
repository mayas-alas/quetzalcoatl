set -eu
install -d -m 0755 \
  /var/lib/quetzalcoatl/proxmox/vz \
  /var/lib/quetzalcoatl/proxmox/cluster
install -d -m 0755 /run/gnx
date --iso-8601=seconds > /run/gnx/proxmox-started-at
systemctl daemon-reload
systemctl stop proxmox.service >/dev/null 2>&1 || true
systemctl reset-failed gnx-node-pod.service proxmox.service >/dev/null 2>&1 || true
if ! systemctl start gnx-node-pod.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 30 -u gnx-node-pod.service >&2 || true
  exit 1
fi
if ! systemctl start proxmox.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 30 -u proxmox.service >&2 || true
  exit 1
fi
systemctl is-active --quiet proxmox.service
printf 'PROXMOX_SERVICE=active\n'
