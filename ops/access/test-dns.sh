#!/bin/sh
# Isolated image/configuration gate: no tailnet or production DNS mutation.
set -eu
repo=$1
image=$(sed -n 's/^Image=//p' "$repo/runtime/access/gnx-dns.container")
work=$(mktemp -d /run/gnx-dns-test.XXXXXX)
container="gnx-dns-test-$$"
cleanup() {
    podman rm -f "$container" >/dev/null 2>&1 || true
    rm -f -- "$work/dns.toml"
    rmdir -- "$work"
}
trap cleanup EXIT HUP INT TERM
sed -e 's/@ZONE@/mesh.gnx/g' -e 's/@IP@/100.100.100.10/g' "$repo/runtime/access/dns.toml" > "$work/dns.toml"
chmod 644 "$work/dns.toml"
podman run -d --name "$container" --network=gnx-control --log-driver=none \
    -p 127.0.0.1::53/udp -p 127.0.0.1::53/tcp \
    --entrypoint=/usr/bin/pihole-FTL \
    -v "$work/dns.toml:/etc/pihole/pihole.toml:ro" "$image" no-daemon >/dev/null
ready=false
for attempt in 1 2 3 4 5; do
    if podman exec "$container" dig @127.0.0.1 mesh.gnx A +short +time=1 +tries=1 2>/dev/null | grep -qx '100.100.100.10'; then
        ready=true; break
    fi
    sleep 1
done
[ "$ready" = true ] || { echo 'FAILED DNS_IMAGE_START'; exit 1; }
for transport in +notcp +tcp; do
    for name in mesh.gnx proxmox.mesh.gnx test.mesh.gnx nested.test.mesh.gnx; do
        answer=$(podman exec "$container" dig @127.0.0.1 "$name" A +short +time=1 +tries=1 "$transport")
        [ "$answer" = '100.100.100.10' ] || { echo 'FAILED DNS_IMAGE_ANSWER'; exit 1; }
    done
    for name in mesh.gnx proxmox.mesh.gnx; do
        answer=$(podman exec "$container" dig @127.0.0.1 "$name" AAAA +time=1 +tries=1 "$transport")
        echo "$answer" | grep -q 'status: NOERROR' || { echo 'FAILED DNS_IMAGE_AAAA'; exit 1; }
        echo "$answer" | grep -q 'ANSWER: 0' || { echo 'FAILED DNS_IMAGE_AAAA'; exit 1; }
    done
    answer=$(podman exec "$container" dig @127.0.0.1 example.com A +time=1 +tries=1 "$transport")
    echo "$answer" | grep -Eq 'status: (REFUSED|SERVFAIL)' || { echo 'FAILED DNS_IMAGE_SCOPE'; exit 1; }
done
for protocol in udp tcp; do
    binding=$(podman port "$container" "53/$protocol")
    case "$binding" in 127.0.0.1:*) ;; *) echo 'FAILED DNS_TEST_BIND'; exit 1 ;; esac
    transport=+notcp
    [ "$protocol" = udp ] || transport=+tcp
    answer=$(podman run --rm --network=host --log-driver=none --entrypoint=/usr/bin/dig \
        "$image" @127.0.0.1 -p "${binding##*:}" mesh.gnx A +short +time=2 +tries=1 "$transport")
    [ "$answer" = '100.100.100.10' ] || { echo 'FAILED DNS_TEST_PUBLISH'; exit 1; }
done
echo 'PASS dns-image udp tcp apex wildcard nested no-aaaa no-global-resolution loopback-publish'
