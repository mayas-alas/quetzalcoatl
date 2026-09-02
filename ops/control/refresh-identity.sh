#!/bin/bash
set -euo pipefail
umask 077
exec 9>/run/gnx-control-maintenance.lock
flock -x 9
state=/var/lib/gnx/control
config="$state/tls.cnf"
openssl x509 -checkend 2592000 -noout -in "$state/tls/root.crt" >/dev/null
renew=false
if ! openssl x509 -checkend 2592000 -noout -in "$state/tls/server.crt" >/dev/null 2>&1 \
    || ! openssl verify -CAfile "$state/tls/root.crt" -verify_hostname proxmox.mesh.gnx "$state/tls/server.crt" >/dev/null 2>&1 \
    || ! openssl x509 -in "$state/tls/server.crt" -noout -ext crlDistributionPoints 2>/dev/null | grep -q 'http://mesh.gnx/pki/root.crl'; then
    renew=true
    if ! test -f "$state/tls/server.key"; then
        openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$state/tls/server.key" 2>/dev/null
    fi
    openssl req -new -key "$state/tls/server.key" -out "$state/pki/server.csr" -subj '/CN=mesh.gnx' 2>/dev/null
    openssl x509 -req -in "$state/pki/server.csr" -CA "$state/tls/root.crt" \
        -CAkey "$state/pki/root.key" -CAcreateserial -out "$state/tls/server.crt" \
        -days 365 -extfile "$config" -extensions server 2>/dev/null
fi
test -f "$state/pki/index.txt" || touch "$state/pki/index.txt"
test -f "$state/pki/crlnumber" || printf '1000\n' > "$state/pki/crlnumber"
openssl ca -gencrl -name local_ca -config "$config" -out "$state/pki/root.crl.pem" 2>/dev/null
openssl crl -in "$state/pki/root.crl.pem" -outform DER -out "$state/public/root.crl.new"
mv "$state/public/root.crl.new" "$state/public/root.crl"
if $renew && test "${1:-}" != '--no-restart' && systemctl --quiet is-active gnx-entry.service; then
    systemctl --no-block try-restart gnx-entry.service
fi
