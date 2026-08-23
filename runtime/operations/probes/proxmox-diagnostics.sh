set -eu
since="$(cat /run/gnx/proxmox-started-at)"
journalctl --no-pager -o cat --since "$since" -u proxmox.service \
  | grep -avE 'image pull|container (create|init|start|died|remove|cleanup)|pod (create|start|stop)' \
  | head -n 60
