set -eu
systemctl daemon-reload
systemctl reset-failed gnx-tailscale-enroll.service tailscaled.service >/dev/null 2>&1 || true
if [ ! -s /var/lib/quetzalcoatl/tailscale/host/tailscaled.state ]; then
  systemctl stop gnx-tailscale-enroll.service >/dev/null 2>&1 || true
fi
if ! systemctl start gnx-tailscale-enroll.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 40 -u gnx-tailscale-enroll.service >&2 || true
  exit 1
fi
test ! -e /run/gnx/ts-authkey
if ! systemctl restart tailscaled.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 40 -u tailscaled.service >&2 || true
  exit 1
fi
systemctl is-active --quiet tailscaled.service
printf 'TAILSCALE_SERVICE=active\n'
