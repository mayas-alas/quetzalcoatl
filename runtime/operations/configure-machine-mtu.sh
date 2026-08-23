set -eu
test -e /sys/class/net/eth0/mtu
ip link set dev eth0 mtu 1500
test "$(cat /sys/class/net/eth0/mtu)" = 1500
printf 'MACHINE_OUTER_MTU=1500\n'
