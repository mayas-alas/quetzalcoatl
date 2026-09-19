# Run from the host network namespace, never from the resolver's own bridge port.
set -eu
ip=$1
port=$2
for transport in +notcp +tcp; do
    for name in mesh.gnx proxmox.mesh.gnx wildcard-check.mesh.gnx; do
        answer=$(dig "@$ip" -p "$port" "$name" A +short +time=2 +tries=1 "$transport")
        test "$answer" = "$3" || exit 1
    done
done
