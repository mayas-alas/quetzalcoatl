#!/usr/bin/env bash
# Control-flow tests with fake native tools; NOT data-plane acceptance.
set -Eeuo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export GNX_TEST_LOG="$tmp/log" GNX_TEST_GUARD_FAIL=0 GNX_TEST_GATE_FAIL=0
mkdir "$tmp/bin"
printf '#!/bin/sh\necho 0\n' > "$tmp/bin/id"
printf '#!/bin/sh\nprintf "nft %%s\\n" "$*" >> "$GNX_TEST_LOG"\n[ "$GNX_TEST_GATE_FAIL" = 0 ]\n' > "$tmp/bin/nft"
printf '#!/bin/sh\nprintf "pct %%s\\n" "$*" >> "$GNX_TEST_LOG"\ncase "$*" in *gnx-workload-mesh*) [ "$GNX_TEST_GUARD_FAIL" = 0 ];; *) exit 0;; esac\n' > "$tmp/bin/pct"
chmod +x "$tmp/bin/"*
export PATH="$tmp/bin:$PATH"
script="$root/payload/node/compute/network.run"
bash "$script" close 100
grep -Fq 'add element netdev gnx_lxc_transport blocked { "veth100i0" }' "$GNX_TEST_LOG"
: > "$GNX_TEST_LOG"
bash "$script" open 100
# Guard must execute before releasing the interface.
guard=$(grep -n 'pct exec 100 -- /usr/local/libexec/gnx-workload-mesh guard' "$GNX_TEST_LOG" | cut -d: -f1)
release=$(grep -n 'delete element' "$GNX_TEST_LOG" | cut -d: -f1)
((guard < release))
: > "$GNX_TEST_LOG"
export GNX_TEST_GUARD_FAIL=1
if bash "$script" open 100 2>/dev/null; then echo 'failed guard accepted' >&2; exit 1; fi
! grep -q 'delete element' "$GNX_TEST_LOG"
export GNX_TEST_GUARD_FAIL=0 GNX_TEST_GATE_FAIL=1
: > "$GNX_TEST_LOG"
if bash "$script" close 100 2>/dev/null; then echo 'failed host gate accepted' >&2; exit 1; fi
grep -Fq 'pct stop 100' "$GNX_TEST_LOG"
: > "$GNX_TEST_LOG"
if bash "$script" close '100;echo injected' 2>/dev/null; then exit 1; fi
[[ ! -s "$GNX_TEST_LOG" ]]
echo 'LXC gate control-flow tests passed (mocked native tools).'
