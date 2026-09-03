#!/bin/sh
# Managed by GNX packaging
set -eu
test "$(id -u)" = 0
bundle=$(realpath "$1")
test -f "$bundle/gnx"
test -d "$bundle/runtime"
install -m 755 "$bundle/gnx" /usr/local/bin/gnx
install -d -m 755 /usr/local/share/gnx /etc/gnx
cp -R "$bundle/runtime" /usr/local/share/gnx/
find /usr/local/share/gnx/runtime -type d -exec chmod 755 {} \;
find /usr/local/share/gnx/runtime -type f -exec chmod 644 {} \;
chmod 755 /usr/local/share/gnx/runtime/access/enroll.sh \
    /usr/local/share/gnx/runtime/compute/entrypoint.sh \
    /usr/local/share/gnx/runtime/controller/ca.sh
if test ! -e /etc/gnx/gnx.toml; then
    install -m 600 "$bundle/gnx.example.toml" /etc/gnx/gnx.toml
fi
echo 'READY linux'
