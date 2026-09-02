#!/bin/bash
set -euo pipefail
umask 077
test "$(id -u)" = 0
templates=$(realpath "$1")
source_state=$(realpath "$2")
state=/var/lib/gnx/control
install -d -m 700 "$state" "$state/pki" "$state/tls" "$state/state" "$state/public"
if test -f "$state/server.yaml"; then
    cmp -s "$source_state/server.yaml" "$state/server.yaml" || {
        printf 'FAILED EXISTING_CONTROL_CONFIG\n'; exit 1;
    }
else
    install -m 600 "$source_state/server.yaml" "$state/server.yaml"
fi
if ! test -f "$state/pki/root.key"; then
    test ! -f "$state/tls/root.crt"
    openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
        -keyout "$state/pki/root.key" -out "$state/tls/root.crt" -days 3650 \
        -subj '/CN=GNX Mesh Local Root' -config "$templates/tls.cnf" -extensions ca 2>/dev/null
fi
install -m 600 "$templates/tls.cnf" "$state/tls.cnf"
install -d -m 755 /usr/local/lib/gnx/control
install -m 755 "$(dirname "$0")/refresh-identity.sh" /usr/local/lib/gnx/control/refresh-identity.sh
/usr/local/lib/gnx/control/refresh-identity.sh
openssl verify -CAfile "$state/tls/root.crt" -verify_hostname mesh.gnx "$state/tls/server.crt"
install -m 600 "$templates/Caddyfile" "$state/Caddyfile"
install -m 600 "$templates/console.env" "$state/console.env"
if ! test -f "$state/bootstrap.env"; then
    printf 'NB_SETUP_PAT_ENABLED=true\n' > "$state/bootstrap.env"
fi
install -d /etc/containers/systemd
for name in gnx-control.network gnx-control.container gnx-console.container gnx-entry.container; do
    install -m 644 "$templates/$name" "/etc/containers/systemd/$name"
done
install -m 644 "$templates/gnx-identity.service" /etc/systemd/system/gnx-identity.service
install -m 644 "$templates/gnx-identity.timer" /etc/systemd/system/gnx-identity.timer
install -m 644 "$state/tls/root.crt" /usr/local/share/ca-certificates/gnx-control.crt
update-ca-certificates >/dev/null
systemctl daemon-reload
systemctl enable --now gnx-identity.timer
systemctl start gnx-control.service gnx-console.service gnx-entry.service
systemctl is-active gnx-control.service gnx-console.service gnx-entry.service
