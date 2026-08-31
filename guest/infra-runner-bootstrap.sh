#!/usr/bin/env bash
set -euo pipefail
umask 077

readonly runner_vmid=200
readonly runner_hostname=gnx-infra-runner
readonly workload_vmid=201
readonly workload_hostname=gnx-cell-01
readonly template_name=ubuntu-24.04-20260826-amd64-root.tar.xz
readonly template_url=https://cloud-images.ubuntu.com/releases/noble/release-20260826/ubuntu-24.04-server-cloudimg-amd64-root.tar.xz
readonly template_sha256=df1146e4f2bc372b193c966b709f1b5e22a5facb27721ad80c5bae254040c380
readonly template_path=/var/lib/vz/template/cache/${template_name}
readonly payload_root=/opt/gnx/guest
readonly token_user=gnx-tofu@pve
readonly token_name=runner
readonly provisioner_privileges='Datastore.Allocate Datastore.AllocateSpace Datastore.AllocateTemplate Datastore.Audit SDN.Use Sys.Audit Sys.Modify VM.Allocate VM.Audit VM.Config.CDROM VM.Config.Cloudinit VM.Config.CPU VM.Config.Disk VM.Config.HWType VM.Config.Memory VM.Config.Network VM.Config.Options VM.PowerMgmt'
temporary=
secret_file=
trap 'rm -f "${temporary:-}" "${secret_file:-}"' EXIT

for command in awk curl grep install od pct perl pvesm pveum seq sha256sum tar tr; do
  command -v "${command}" >/dev/null || {
    echo "GNX: falta ${command} dentro de Proxmox" >&2
    exit 69
  }
done

pvesm set local --content iso,vztmpl,backup,rootdir,images,snippets
install -d -m 0755 "$(dirname "${template_path}")"
if ! echo "${template_sha256}  ${template_path}" | sha256sum --check --status; then
  temporary="$(mktemp "${template_path}.partial.XXXXXX")"
  curl --fail --location --proto '=https' --tlsv1.2 --output "${temporary}" "${template_url}"
  echo "${template_sha256}  ${temporary}" | sha256sum --check --status
  chmod 0644 "${temporary}"
  mv -f "${temporary}" "${template_path}"
fi

if pct config "${runner_vmid}" >/dev/null 2>&1; then
  actual_hostname="$(pct config "${runner_vmid}" | awk -F ': ' '$1 == "hostname" { print $2 }')"
  if [[ "${actual_hostname}" != "${runner_hostname}" ]]; then
    echo "GNX: VMID ${runner_vmid} pertenece a ${actual_hostname:-otro recurso}; no se modifica" >&2
    exit 70
  fi
else
  pct create "${runner_vmid}" "local:vztmpl/${template_name}" \
    --hostname "${runner_hostname}" \
    --rootfs local:12 \
    --cores 1 \
    --memory 1024 \
    --swap 256 \
    --unprivileged 1 \
    --features nesting=1,keyctl=1 \
    --net0 name=eth0,bridge=vmbr0,ip=dhcp,type=veth \
    --onboot 1 \
    --startup order=5,up=10,down=30
fi

if ! pct status "${runner_vmid}" | grep -q 'status: running'; then
  pct start "${runner_vmid}"
fi

for _ in $(seq 1 60); do
  if pct exec "${runner_vmid}" -- /bin/true >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
pct exec "${runner_vmid}" -- /bin/true

pct exec "${runner_vmid}" -- install -d -m 0700 /etc/gnx /var/lib/gnx/opentofu
pct exec "${runner_vmid}" -- install -d -m 0755 /opt/gnx/infra /usr/local/libexec
if ! pct exec "${runner_vmid}" -- test -f /var/lib/gnx/opentofu/bootstrap-v1; then
  pct exec "${runner_vmid}" -- env DEBIAN_FRONTEND=noninteractive apt-get update
  pct exec "${runner_vmid}" -- env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates iproute2
  pct exec "${runner_vmid}" -- touch /var/lib/gnx/opentofu/bootstrap-v1
fi
pct push "${runner_vmid}" "${payload_root}/opentofu.tar.gz" /var/tmp/gnx-opentofu.tar.gz
pct exec "${runner_vmid}" -- tar -xzf /var/tmp/gnx-opentofu.tar.gz -C /usr/local/bin tofu
pct exec "${runner_vmid}" -- chmod 0755 /usr/local/bin/tofu
pct exec "${runner_vmid}" -- rm -f /var/tmp/gnx-opentofu.tar.gz

for file in versions.tf variables.tf main.tf outputs.tf .terraform.lock.hcl; do
  pct push "${runner_vmid}" "${payload_root}/opentofu/${file}" "/opt/gnx/infra/${file}"
done
pct push "${runner_vmid}" "${payload_root}/infra-runner-run.sh" /usr/local/libexec/gnx-opentofu-run
pct push "${runner_vmid}" "${payload_root}/units/gnx-opentofu.service" /etc/systemd/system/gnx-opentofu.service
pct exec "${runner_vmid}" -- chmod 0755 /usr/local/libexec/gnx-opentofu-run
pct exec "${runner_vmid}" -- chmod 0644 /etc/systemd/system/gnx-opentofu.service

if ! pct exec "${runner_vmid}" -- test -s /etc/gnx/opentofu.env; then
  pveum user add "${token_user}" --comment 'GNX dedicated OpenTofu runner' 2>/dev/null || \
    pveum user list | grep -Fq "${token_user}"
  pveum role add GNXProvisioner --privs "${provisioner_privileges}" 2>/dev/null || \
    pveum role list | grep -Fq GNXProvisioner
  pveum role modify GNXProvisioner --privs "${provisioner_privileges}"
  pveum aclmod / --user "${token_user}" --role GNXProvisioner --propagate 1
  pveum user token remove "${token_user}" "${token_name}" >/dev/null 2>&1 || true
  token_json="$(pveum user token add "${token_user}" "${token_name}" --privsep 0 --output-format json)"
  token_value="$(printf '%s' "${token_json}" | perl -MJSON::PP -0777 -ne '$d=decode_json($_); print $d->{value}')"
  if [[ -z "${token_value}" ]]; then
    echo 'GNX: Proxmox no devolvió el secreto del token dedicado' >&2
    exit 71
  fi
  guest_password="$(od -An -N32 -tx1 /dev/urandom | tr -d ' \n')"
  secret_file="$(mktemp /run/gnx-opentofu.XXXXXX)"
  printf 'PROXMOX_VE_API_TOKEN=%s!%s=%s\nTF_VAR_guest_password=%s\n' \
    "${token_user}" "${token_name}" "${token_value}" "${guest_password}" > "${secret_file}"
  pct push "${runner_vmid}" "${secret_file}" /etc/gnx/opentofu.env
  pct exec "${runner_vmid}" -- chmod 0600 /etc/gnx/opentofu.env
  rm -f "${secret_file}"
fi

pct exec "${runner_vmid}" -- systemctl daemon-reload
pct exec "${runner_vmid}" -- systemctl enable gnx-opentofu.service
pct exec "${runner_vmid}" -- systemctl restart gnx-opentofu.service

actual_workload_hostname="$(pct config "${workload_vmid}" | awk -F ': ' '$1 == "hostname" { print $2 }')"
if [[ "${actual_workload_hostname}" != "${workload_hostname}" ]]; then
  echo "GNX: VMID ${workload_vmid} no corresponde al workload administrado" >&2
  exit 72
fi
if ! pct status "${workload_vmid}" | grep -q 'status: running'; then
  pct start "${workload_vmid}"
fi
for _ in $(seq 1 60); do
  if pct exec "${workload_vmid}" -- /bin/true >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
pct exec "${workload_vmid}" -- /bin/true
pct exec "${workload_vmid}" -- install -d -m 0755 /opt/gnx/guest/units
pct exec "${workload_vmid}" -- install -d -m 0700 /run/gnx/mesh
pct push "${workload_vmid}" "${payload_root}/bootstrap.sh" /opt/gnx/guest/bootstrap.sh
pct push "${workload_vmid}" "${payload_root}/tailscale-controller.env" /opt/gnx/guest/tailscale-controller.env
pct push "${workload_vmid}" "${payload_root}/units/tailscale.container" /opt/gnx/guest/units/tailscale.container
pct push "${workload_vmid}" "${payload_root}/units/docktail.container" /opt/gnx/guest/units/docktail.container
pct exec "${workload_vmid}" -- chmod 0755 /opt/gnx/guest/bootstrap.sh
pct exec "${workload_vmid}" -- chmod 0600 /opt/gnx/guest/tailscale-controller.env
pct exec "${workload_vmid}" -- chmod 0644 /opt/gnx/guest/units/tailscale.container /opt/gnx/guest/units/docktail.container
if [[ -s /run/gnx/mesh/auth.key ]]; then
  pct push "${workload_vmid}" /run/gnx/mesh/auth.key /run/gnx/mesh/auth.key
  pct exec "${workload_vmid}" -- chmod 0400 /run/gnx/mesh/auth.key
fi
pct exec "${workload_vmid}" -- /opt/gnx/guest/bootstrap.sh
