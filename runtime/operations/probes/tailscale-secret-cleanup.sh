set -eu
test ! -e /run/gnx/ts-authkey
test -z "$(podman ps -aq --filter name='^gnx-host-enroll$')"
printf 'TAILSCALE_SECRET_CLEAN=ready\n'
