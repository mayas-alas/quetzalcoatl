#!/usr/bin/env bash
set -euo pipefail

set -a
source /etc/gnx/opentofu.env
set +a

gateway="$(ip -4 route show default | awk 'NR == 1 { print $3 }')"
if [[ -z "${gateway}" ]]; then
  echo "GNX: el runner no pudo resolver el gateway de Proxmox" >&2
  exit 68
fi

export PROXMOX_VE_ENDPOINT="https://${gateway}:8006/"
export PROXMOX_VE_INSECURE=true
export TF_DATA_DIR=/var/lib/gnx/opentofu/.terraform
export TF_IN_AUTOMATION=1

cd /opt/gnx/infra
/usr/local/bin/tofu init -input=false -lockfile=readonly
/usr/local/bin/tofu validate
/usr/local/bin/tofu apply -auto-approve -input=false -lock-timeout=5m
