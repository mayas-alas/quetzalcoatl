#!/bin/sh
# Managed by GNX compute
set -eu
unset PASSWORD PASSWORD_HASH
test -s /run/gnx/password
PASSWORD=$(cat /run/gnx/password)
export PASSWORD
exec /usr/local/bin/entrypoint.sh "$@"
