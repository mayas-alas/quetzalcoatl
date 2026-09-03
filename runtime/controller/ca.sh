#!/bin/sh
# Managed by GNX controller; creates a local CA without trusting it on any client.
set -eu
test "$#" -ge 2
state=$1
shift
umask 077
mkdir -p "$state/pki" "$state/tls" "$state/public"

root_key="$state/pki/root.key"
root_crt="$state/public/root.crt"
server_key="$state/tls/server.key"
server_crt="$state/tls/server.crt"

if { test -e "$root_key" && test ! -e "$root_crt"; } || { test ! -e "$root_key" && test -e "$root_crt"; }; then
    echo 'FAILED CA_IDENTITY' >&2
    exit 1
fi
if test ! -e "$root_key"; then
    openssl req -x509 -newkey rsa:3072 -sha256 -days 3650 -nodes \
        -subj '/CN=GNX Autonomous Root' \
        -addext 'basicConstraints=critical,CA:TRUE,pathlen:0' \
        -addext 'keyUsage=critical,keyCertSign,cRLSign' \
        -addext 'nameConstraints=critical,permitted;DNS:.gnx' \
        -keyout "$root_key" -out "$root_crt" >/dev/null 2>&1
fi

san=
for host in "$@"; do
    case "$host" in *[!A-Za-z0-9.-]*|'') exit 2;; esac
    san="${san:+$san,}DNS:$host"
done
if test ! -s "$server_crt" || ! openssl x509 -checkend 2592000 -noout -in "$server_crt" >/dev/null 2>&1; then
    csr="$state/pki/server.csr"
    openssl req -new -newkey rsa:2048 -sha256 -nodes \
        -subj '/CN=GNX Private Services' -addext "subjectAltName=$san" \
        -keyout "$server_key" -out "$csr" >/dev/null 2>&1
    openssl x509 -req -sha256 -days 397 -in "$csr" \
        -CA "$root_crt" -CAkey "$root_key" -CAcreateserial \
        -copy_extensions copy -out "$server_crt" >/dev/null 2>&1
    rm -f -- "$csr"
fi
chmod 600 "$root_key" "$server_key"
chmod 644 "$root_crt" "$server_crt"
