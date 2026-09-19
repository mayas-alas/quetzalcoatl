#!/bin/bash
set -euo pipefail
# The pinned entrypoint reads a shell-local variable, never an exported environment value.
unset PASSWORD PASSWORD_HASH
test -s /run/gnx/password
PASSWORD=$(</run/gnx/password)
test "${#PASSWORD}" -ge 16
source /usr/local/bin/entrypoint.sh "$@"
