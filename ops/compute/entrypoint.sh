#!/bin/sh
set -eu
unset PASSWORD PASSWORD_HASH
test -s /run/gnx/password
PASSWORD=$(cat /run/gnx/password)
test "${#PASSWORD}" -ge 16
. /usr/local/bin/entrypoint.sh "$@"
