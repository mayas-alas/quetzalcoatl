set -eu
test "$(ps -p 1 -o comm= | tr -d ' ')" = systemd
test "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs
systemctl is-system-running --wait >/dev/null 2>&1 || test "$(systemctl is-system-running)" = degraded
printf 'SYSTEMD=ready;CGROUP=ready\n'
