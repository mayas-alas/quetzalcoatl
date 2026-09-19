#!/bin/sh
# No real key and no provider connection: exercise temporary credential custody.
set -eu
repo=$1
image=$(sed -n 's/^Image=//p' "$repo/runtime/access/gnx-access.container")
podman run --rm --network=none --log-driver=none --entrypoint=/bin/sh \
    --tmpfs=/run/gnx:rw,nosuid,nodev,noexec,size=1m,mode=0700 \
    -v "$repo/runtime/access/enroll.sh:/test/enroll.sh:ro" "$image" -ec '
    test "$(stat -f -c %T /run/gnx)" = tmpfs
    for expected in 0 42; do
        result=0
        printf %s GNX-NONSECRET-PROBE | sh /test/enroll.sh sh -ec '\''
            key=${2#--auth-key=file:}
            test -f "$key"
            test "$(stat -c %a "$key")" = 600
            test "$(cat "$key")" = GNX-NONSECRET-PROBE
            exit "$1"
        '\'' gnx "$expected" || result=$?
        test "$result" -eq "$expected"
        test "$(find /run/gnx -name "enrollment.*" | wc -l)" -eq 0
    done
    echo "PASS enrollment-stdin tmpfs mode-600 cleanup-success cleanup-failure"
    '
