#!/usr/bin/env bash
set -euo pipefail

marker=/var/lib/gnx/bootstrap-v1
if [[ ! -f "${marker}" ]]; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update
  apt-get install -y --no-install-recommends ca-certificates podman
  touch "${marker}"
fi

install -d -m 0755 /etc/containers/systemd /var/lib/gnx /run/gnx/tailscale
install -d -m 0700 /etc/gnx /run/gnx/mesh
install -m 0600 /opt/gnx/guest/tailscale-controller.env /etc/gnx/tailscale-controller.env
if [[ -s /run/gnx/mesh/auth.key ]]; then
  printf 'TS_AUTHKEY=file:/run/secrets/gnx/auth.key\n' > /run/gnx/mesh/tailscale-auth.env
else
  : > /run/gnx/mesh/tailscale-auth.env
fi
chmod 0600 /run/gnx/mesh/tailscale-auth.env
install -m 0644 /opt/gnx/guest/units/tailscale.container /etc/containers/systemd/tailscale.container
install -m 0644 /opt/gnx/guest/units/docktail.container /etc/containers/systemd/docktail.container

systemctl daemon-reload
systemctl enable --now podman.socket tailscale.service
for _ in $(seq 1 60); do
  status="$(podman exec gnx-tailscale tailscale --socket=/var/run/tailscale/tailscaled.sock status --json 2>/dev/null || true)"
  if grep -Eq '"BackendState"[[:space:]]*:[[:space:]]*"Running"' <<<"${status}" &&
     ! grep -Eq '"TailscaleIPs"[[:space:]]*:[[:space:]]*\[[[:space:]]*\]' <<<"${status}"; then
    break
  fi
  sleep 2
done
status="$(podman exec gnx-tailscale tailscale --socket=/var/run/tailscale/tailscaled.sock status --json)"
grep -Eq '"BackendState"[[:space:]]*:[[:space:]]*"Running"' <<<"${status}"
! grep -Eq '"TailscaleIPs"[[:space:]]*:[[:space:]]*\[[[:space:]]*\]' <<<"${status}"
rm -f /run/gnx/mesh/auth.key
: > /run/gnx/mesh/tailscale-auth.env
systemctl enable --now docktail.service
systemctl is-active --quiet docktail.service
