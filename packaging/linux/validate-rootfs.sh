#!/bin/sh
set -eu

usage() {
    cat >&2 <<'EOF'
Usage: validate-rootfs.sh --rootfs rootfs.tar --sha256 SHA256 --source ID --version VERSION --arch amd64 [--metadata OUT]
Validates a locally supplied GNX Wide Linux rootfs tar and writes sanitized metadata.
EOF
    exit 64
}

fail() {
    printf '%s\n' "$1" >&2
    exit 1
}

rootfs=
expected_sha=
source_id=
version_id=
arch=
metadata=
max_bytes=${GNX_ROOTFS_MAX_BYTES:-4294967296}
min_bytes=${GNX_ROOTFS_MIN_BYTES:-10240}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --rootfs) [ "$#" -ge 2 ] || usage; rootfs=$2; shift 2 ;;
        --sha256) [ "$#" -ge 2 ] || usage; expected_sha=$2; shift 2 ;;
        --source) [ "$#" -ge 2 ] || usage; source_id=$2; shift 2 ;;
        --version) [ "$#" -ge 2 ] || usage; version_id=$2; shift 2 ;;
        --arch) [ "$#" -ge 2 ] || usage; arch=$2; shift 2 ;;
        --metadata) [ "$#" -ge 2 ] || usage; metadata=$2; shift 2 ;;
        -h|--help) usage ;;
        *) usage ;;
    esac
done

[ -n "$rootfs" ] || fail 'ROOTFS_MISSING'
[ -n "$expected_sha" ] || fail 'ROOTFS_EXPECTED_HASH_MISSING'
[ -n "$source_id" ] || fail 'ROOTFS_SOURCE_MISSING'
[ -n "$version_id" ] || fail 'ROOTFS_VERSION_MISSING'
[ -n "$arch" ] || fail 'ROOTFS_ARCH_MISSING'

case "$expected_sha" in
    *[!0123456789abcdefABCDEF]*|'') fail 'ROOTFS_EXPECTED_HASH_INVALID' ;;
esac
[ ${#expected_sha} -eq 64 ] || fail 'ROOTFS_EXPECTED_HASH_INVALID'

case "$source_id" in
    *://*|*'?'*|*'&'*|*'='*|*\\*|'') fail 'ROOTFS_SOURCE_UNSAFE' ;;
esac
case "$version_id" in
    *://*|*'?'*|*'&'*|*'='*|*\\*|'') fail 'ROOTFS_VERSION_UNSAFE' ;;
esac
case "$source_id$version_id" in
    *[!A-Za-z0-9._:@+/-]*) fail 'ROOTFS_METADATA_UNSAFE' ;;
esac
[ "$arch" = amd64 ] || fail 'ROOTFS_ARCH_UNSUPPORTED'
case "$max_bytes$min_bytes" in *[!0-9]*|'') fail 'ROOTFS_LIMIT_INVALID' ;; esac

[ -f "$rootfs" ] || fail 'ROOTFS_NOT_FOUND'
[ ! -L "$rootfs" ] || fail 'ROOTFS_PATH_INVALID'
case "${rootfs##*/}" in *.tar) ;; *) fail 'ROOTFS_FORMAT_INVALID' ;; esac

size=$(wc -c < "$rootfs" | tr -d ' ')
[ "$size" -ge "$min_bytes" ] || fail 'ROOTFS_TOO_SMALL'
[ "$size" -le "$max_bytes" ] || fail 'ROOTFS_TOO_LARGE'

actual_sha=$(sha256sum "$rootfs" | awk '{print $1}')
[ "$actual_sha" = "$(printf '%s' "$expected_sha" | tr 'A-F' 'a-f')" ] || fail 'ROOTFS_HASH_MISMATCH'

listing=$(mktemp)
trap 'rm -f "$listing"' EXIT
tar -tf "$rootfs" > "$listing" 2>/dev/null || fail 'ROOTFS_TAR_INVALID'

# Refuse tar paths that could escape an import root.
if grep -Eq '(^/|(^|/)\.\.(/|$))' "$listing"; then
    fail 'ROOTFS_TAR_PATH_INVALID'
fi

has_entry() {
    grep -Eq "^(\./)?$1(/)?$" "$listing"
}

has_any() {
    for entry in "$@"; do
        if has_entry "$entry"; then
            return 0
        fi
    done
    return 1
}

has_any etc/os-release usr/lib/os-release || fail 'ROOTFS_OS_RELEASE_MISSING'
has_any bin/sh usr/bin/sh || fail 'ROOTFS_COMMAND_MISSING_SH'
has_any usr/bin/env bin/env || fail 'ROOTFS_COMMAND_MISSING_ENV'
has_any usr/bin/tar bin/tar || fail 'ROOTFS_COMMAND_MISSING_TAR'
has_any usr/bin/id bin/id || fail 'ROOTFS_COMMAND_MISSING_ID'
has_any usr/bin/mkdir bin/mkdir || fail 'ROOTFS_COMMAND_MISSING_MKDIR'
has_any usr/bin/chmod bin/chmod || fail 'ROOTFS_COMMAND_MISSING_CHMOD'
has_any usr/bin/cp bin/cp || fail 'ROOTFS_COMMAND_MISSING_CP'
has_any usr/bin/rm bin/rm || fail 'ROOTFS_COMMAND_MISSING_RM'
has_any usr/bin/timeout bin/timeout || fail 'ROOTFS_COMMAND_MISSING_TIMEOUT'
has_any usr/bin/curl bin/curl || fail 'ROOTFS_COMMAND_MISSING_CURL'
has_any usr/bin/openssl bin/openssl || fail 'ROOTFS_COMMAND_MISSING_OPENSSL'
has_any usr/bin/podman bin/podman || fail 'ROOTFS_COMMAND_MISSING_PODMAN'
has_any usr/lib/systemd/systemd lib/systemd/systemd || fail 'ROOTFS_SYSTEMD_MISSING'
has_any etc/passwd || fail 'ROOTFS_PASSWD_MISSING'
has_any var/lib/dpkg/arch etc/apk/arch usr/lib/rpm/platform || fail 'ROOTFS_ARCH_WITNESS_MISSING'

arch_text=$( (tar -xOf "$rootfs" ./var/lib/dpkg/arch 2>/dev/null || tar -xOf "$rootfs" var/lib/dpkg/arch 2>/dev/null || tar -xOf "$rootfs" ./etc/apk/arch 2>/dev/null || tar -xOf "$rootfs" etc/apk/arch 2>/dev/null || tar -xOf "$rootfs" ./usr/lib/rpm/platform 2>/dev/null || tar -xOf "$rootfs" usr/lib/rpm/platform 2>/dev/null || true) | head -n 1 )
case "$arch_text" in
    amd64|x86_64|x86_64-*) ;;
    *) fail 'ROOTFS_ARCH_WITNESS_MISMATCH' ;;
esac

metadata_json='{"schema":1,"kind":"gnx-rootfs","source":"'"$source_id"'","version":"'"$version_id"'","architecture":"amd64","sha256":"'"$actual_sha"'","size_bytes":'"$size"',"format":"posix-tar","requirements":["etc/os-release","/bin/sh or /usr/bin/sh","env","tar","id","mkdir","chmod","cp","rm","timeout","curl","openssl","podman","systemd","etc/passwd","amd64 architecture witness"],"reproducibility":"not_claimed"}'
if [ -n "$metadata" ]; then
    umask 077
    tmp=${metadata}.$$.tmp
    printf '%s\n' "$metadata_json" > "$tmp"
    mv "$tmp" "$metadata"
else
    printf '%s\n' "$metadata_json"
fi
printf '%s\n' 'ROOTFS_VALIDATED' >&2
