#!/bin/bash
set -euo pipefail
# Source the pinned upstream entrypoint with a shell-local password, never env/argv.
unset PASSWORD PASSWORD_HASH
test -s /run/gnx/password
PASSWORD=$(</run/gnx/password)
test "${#PASSWORD}" -ge 40
source /usr/local/bin/entrypoint.sh "$@"
