#!/usr/bin/env bash
set -euo pipefail

marker=/var/lib/gnx/bootstrap-v1
if [[ ! -f "${marker}" ]]; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update
  apt-get install -y --no-install-recommends ca-certificates podman
  install -d -m 0755 /etc/containers/systemd /var/lib/gnx /run/gnx/tailscale
  install -d -m 0700 /etc/gnx
  install -m 0600 /opt/gnx/guest/tailscale-controller.env /etc/gnx/tailscale-controller.env
  : > /etc/gnx/tailscale-auth.env
  chmod 0600 /etc/gnx/tailscale-auth.env
  install -m 0644 /opt/gnx/guest/units/tailscale.container /etc/containers/systemd/tailscale.container
  install -m 0644 /opt/gnx/guest/units/docktail.container /etc/containers/systemd/docktail.container
  touch "${marker}"
fi

systemctl daemon-reload
systemctl enable --now podman.socket tailscale.service docktail.service
