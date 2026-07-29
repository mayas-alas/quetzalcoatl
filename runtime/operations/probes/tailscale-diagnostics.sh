set -eu
journalctl --no-pager -o cat -r -n 30 \
  -u gnx-tailscale-enroll.service -u tailscaled.service 2>/dev/null \
  | head -n 60
