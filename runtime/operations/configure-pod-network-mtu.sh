set -eu
bridge=podman0
test -d "/sys/class/net/$bridge/brif"
ip link set dev "$bridge" mtu 1500
members=0
for member_path in "/sys/class/net/$bridge/brif/"*; do
  test -e "$member_path"
  member=${member_path##*/}
  ip link set dev "$member" mtu 1500
  test "$(cat "/sys/class/net/$member/mtu")" = 1500
  members=$((members + 1))
done
test "$members" -ge 1
test "$(cat "/sys/class/net/$bridge/mtu")" = 1500
test "$(podman exec gnx-proxmox cat /sys/class/net/eth0/mtu)" = 1500
printf 'POD_NETWORK_MTU=1500;MEMBERS=%s\n' "$members"
