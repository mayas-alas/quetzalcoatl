#!/usr/bin/env bash
set -euo pipefail
umask 077

readonly control_vmid=190
readonly control_hostname=gnx-control-plane
readonly control_address=172.30.70.10
readonly control_gateway=172.30.70.1
readonly template_name=ubuntu-24.04-20260826-amd64-root.tar.xz
readonly template_url=https://cloud-images.ubuntu.com/releases/noble/release-20260826/ubuntu-24.04-server-cloudimg-amd64-root.tar.xz
readonly template_sha256=df1146e4f2bc372b193c966b709f1b5e22a5facb27721ad80c5bae254040c380
readonly template_path=/var/lib/vz/template/cache/${template_name}
readonly payload_root=/opt/gnx/guest
temporary=
trap 'rm -f "${temporary:-}"' EXIT

forward_control_plane() {
  iptables -t nat -C PREROUTING -p tcp --dport 443 -j DNAT --to-destination "${control_address}:443" 2>/dev/null ||
    iptables -t nat -A PREROUTING -p tcp --dport 443 -j DNAT --to-destination "${control_address}:443"
  iptables -C FORWARD -p tcp -d "${control_address}" --dport 443 -j ACCEPT 2>/dev/null ||
    iptables -A FORWARD -p tcp -d "${control_address}" --dport 443 -j ACCEPT
}

issue_key() {
  case "${1:-}" in
    runtime) tags=tag:server,tag:gnx-runtime ;;
    cell) tags=tag:server,tag:gnx-cell ;;
    *) echo 'GNX: rol de credencial mesh inválido' >&2; exit 64 ;;
  esac
  pct exec "${control_vmid}" -- podman exec gnx-headscale headscale preauthkeys create --tags "${tags}"
}

if [[ "${1:-bootstrap}" == forward ]]; then
  forward_control_plane
  exit 0
fi
if [[ "${1:-bootstrap}" == issue-key ]]; then
  issue_key "${2:-}"
  exit 0
fi
if [[ "${1:-bootstrap}" != bootstrap ]]; then
  echo 'GNX: acción de control plane inválida' >&2
  exit 64
fi

for command in awk curl grep install iptables pct pvesm seq sha256sum; do
  command -v "${command}" >/dev/null || {
    echo "GNX: falta ${command} dentro de Proxmox" >&2
    exit 69
  }
done

if ! ip -4 address show dev vmbr0 | grep -Fq "${control_gateway}/24"; then
  echo "GNX: la red LXC esperada ${control_gateway}/24 no está disponible en vmbr0" >&2
  exit 73
fi
forward_control_plane

pvesm set local --content iso,vztmpl,backup,rootdir,images,snippets
install -d -m 0755 "$(dirname "${template_path}")"
if ! echo "${template_sha256}  ${template_path}" | sha256sum --check --status; then
  temporary="$(mktemp "${template_path}.partial.XXXXXX")"
  curl --fail --location --proto '=https' --tlsv1.2 --output "${temporary}" "${template_url}"
  echo "${template_sha256}  ${temporary}" | sha256sum --check --status
  chmod 0644 "${temporary}"
  mv -f "${temporary}" "${template_path}"
fi

if pct config "${control_vmid}" >/dev/null 2>&1; then
  actual_hostname="$(pct config "${control_vmid}" | awk -F ': ' '$1 == "hostname" { print $2 }')"
  if [[ "${actual_hostname}" != "${control_hostname}" ]]; then
    echo "GNX: VMID ${control_vmid} pertenece a ${actual_hostname:-otro recurso}; no se modifica" >&2
    exit 70
  fi
else
  pct create "${control_vmid}" "local:vztmpl/${template_name}" \
    --hostname "${control_hostname}" \
    --rootfs local:8 \
    --cores 1 \
    --memory 768 \
    --swap 256 \
    --unprivileged 1 \
    --features nesting=1,keyctl=1 \
    --net0 "name=eth0,bridge=vmbr0,ip=${control_address}/24,gw=${control_gateway},type=veth" \
    --onboot 1 \
    --startup order=1,up=10,down=30
fi

if ! pct status "${control_vmid}" | grep -q 'status: running'; then
  pct start "${control_vmid}"
fi
for _ in $(seq 1 60); do
  if pct exec "${control_vmid}" -- /bin/true >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
pct exec "${control_vmid}" -- /bin/true

pct exec "${control_vmid}" -- install -d -m 0755 /etc/containers/systemd /etc/gnx/headscale /var/lib/gnx
pct exec "${control_vmid}" -- install -d -m 0700 /var/lib/gnx/headscale /var/lib/gnx/headscale/tls
if ! pct exec "${control_vmid}" -- test -f /var/lib/gnx/headscale/bootstrap-v1; then
  pct exec "${control_vmid}" -- env DEBIAN_FRONTEND=noninteractive apt-get update
  pct exec "${control_vmid}" -- env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates curl openssl podman
  pct exec "${control_vmid}" -- touch /var/lib/gnx/headscale/bootstrap-v1
fi

pct push "${control_vmid}" "${payload_root}/headscale-config.yaml" /etc/gnx/headscale/config.yaml
pct push "${control_vmid}" "${payload_root}/headscale-policy.hujson" /etc/gnx/headscale/policy.hujson
pct push "${control_vmid}" "${payload_root}/units/headscale.container" /etc/containers/systemd/headscale.container
pct exec "${control_vmid}" -- chmod 0644 /etc/gnx/headscale/config.yaml /etc/gnx/headscale/policy.hujson /etc/containers/systemd/headscale.container

if ! pct exec "${control_vmid}" -- test -s /var/lib/gnx/headscale/tls/ca.crt; then
  pct exec "${control_vmid}" -- openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:3072 -out /var/lib/gnx/headscale/tls/ca.key
  pct exec "${control_vmid}" -- openssl req -x509 -new -sha256 -days 3650 \
    -key /var/lib/gnx/headscale/tls/ca.key \
    -subj /CN=Quetzalcoatl-Next-Control-Plane-CA \
    -out /var/lib/gnx/headscale/tls/ca.crt
  pct exec "${control_vmid}" -- openssl req -new -newkey rsa:3072 -nodes \
    -keyout /var/lib/gnx/headscale/tls/server.key \
    -subj /CN=controlplane.node.gnx \
    -addext subjectAltName=DNS:controlplane.node.gnx,DNS:headscale.node.gnx,IP:172.30.70.10 \
    -out /var/lib/gnx/headscale/tls/server.csr
  pct exec "${control_vmid}" -- openssl x509 -req -sha256 -days 825 \
    -in /var/lib/gnx/headscale/tls/server.csr \
    -CA /var/lib/gnx/headscale/tls/ca.crt \
    -CAkey /var/lib/gnx/headscale/tls/ca.key \
    -CAcreateserial -copy_extensions copy \
    -out /var/lib/gnx/headscale/tls/server.crt
  pct exec "${control_vmid}" -- rm -f /var/lib/gnx/headscale/tls/server.csr
fi
pct exec "${control_vmid}" -- chmod 0600 /var/lib/gnx/headscale/tls/ca.key /var/lib/gnx/headscale/tls/server.key
pct exec "${control_vmid}" -- chmod 0644 /var/lib/gnx/headscale/tls/ca.crt /var/lib/gnx/headscale/tls/server.crt

pct exec "${control_vmid}" -- systemctl daemon-reload
pct exec "${control_vmid}" -- systemctl enable --now headscale.service
for _ in $(seq 1 60); do
  if pct exec "${control_vmid}" -- curl --fail --silent --show-error \
    --cacert /var/lib/gnx/headscale/tls/ca.crt \
    --resolve controlplane.node.gnx:443:172.30.70.10 \
    https://controlplane.node.gnx/health >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
pct exec "${control_vmid}" -- podman wait --condition=healthy --interval=2s gnx-headscale >/dev/null
install -d -m 0755 /run/gnx/control-plane
pct pull "${control_vmid}" /var/lib/gnx/headscale/tls/ca.crt /run/gnx/control-plane/ca.crt
chmod 0644 /run/gnx/control-plane/ca.crt
