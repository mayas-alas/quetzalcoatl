#!/bin/bash
set -euo pipefail
umask 077
test "$(id -u)" = 0
exec 9>/run/gnx-control-maintenance.lock
flock -x 9
systemctl is-active --quiet gnx-control.service
stage=$(mktemp -d /run/gnx-snapshot.XXXXXX)
stopped=false
cleanup() {
    code=$?
    if $stopped; then systemctl start gnx-control.service >&2 || code=1; fi
    case "$stage" in
        /run/gnx-snapshot.*)
            if test "$(realpath "$stage")" = "$stage"; then rm -rf -- "$stage"; else code=1; fi ;;
        *) code=1 ;;
    esac
    exit "$code"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
stopped=true
systemctl stop gnx-control.service
cp -a /var/lib/gnx/control "$stage/control"
systemctl start gnx-control.service >&2
stopped=false
mkdir "$stage/services" "$stage/ops"
for file in gnx-control.network gnx-control.container gnx-console.container gnx-entry.container; do
    cp -a "/etc/containers/systemd/$file" "$stage/services/"
done
cp -a /etc/systemd/system/gnx-identity.service /etc/systemd/system/gnx-identity.timer "$stage/services/"
cp -a /usr/local/lib/gnx/control/refresh-identity.sh "$stage/ops/"
tar -czf - -C "$stage" control services ops
