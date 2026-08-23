set -eu
test "$(podman inspect --format '{{.State.Status}}' gnx-proxmox)" = running
test "$(podman exec gnx-proxmox ps -p 1 -o comm= | tr -d ' ')" = systemd
test "$(podman exec gnx-proxmox stat -fc %T /sys/fs/cgroup)" = cgroup2fs
podman exec gnx-proxmox systemctl is-active --quiet pve-cluster.service
podman exec gnx-proxmox systemctl is-active --quiet pvedaemon.service
podman exec gnx-proxmox systemctl is-active --quiet pveproxy.service
podman exec gnx-proxmox pvesh get /version --output-format json >/dev/null
printf 'PVE=ready;SYSTEMD=ready;CGROUP=ready\n'
