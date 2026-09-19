#!/bin/sh
set -eu

script=${1:-packaging/linux/validate-rootfs.sh}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

make_rootfs() {
    root=$1
    tarball=$2
    mkdir -p "$root/bin" "$root/usr/bin" "$root/usr/lib/systemd" "$root/etc" "$root/var/lib/dpkg"
    : > "$root/bin/sh"
    : > "$root/usr/bin/env"
    : > "$root/usr/bin/tar"
    : > "$root/usr/bin/id"
    : > "$root/usr/bin/mkdir"
    : > "$root/usr/bin/chmod"
    : > "$root/usr/bin/cp"
    : > "$root/usr/bin/rm"
    : > "$root/usr/bin/timeout"
    : > "$root/usr/bin/curl"
    : > "$root/usr/bin/openssl"
    : > "$root/usr/bin/podman"
    : > "$root/usr/lib/systemd/systemd"
    printf 'ID=gnx-test\nVERSION_ID=0\n' > "$root/etc/os-release"
    printf 'root:x:0:0:root:/root:/bin/sh\n' > "$root/etc/passwd"
    printf 'Package: gnx-test\nArchitecture: amd64\n' > "$root/var/lib/dpkg/status"
    tar -C "$root" -cf "$tarball" .
}

root=$work/root
mkdir -p "$root"
rootfs=$work/rootfs.tar
make_rootfs "$root" "$rootfs"
sha=$(sha256sum "$rootfs" | awk '{print $1}')
meta=$work/meta.json
"$script" --rootfs "$rootfs" --sha256 "$sha" --source gnx-test-rootfs --version 0.0.0 --arch amd64 --metadata "$meta" >/dev/null

grep -q '"kind":"gnx-rootfs"' "$meta"
grep -q '"reproducibility":"not_claimed"' "$meta"

if "$script" --rootfs "$rootfs" --sha256 "0000000000000000000000000000000000000000000000000000000000000000" --source gnx-test-rootfs --version 0.0.0 --arch amd64 >/dev/null 2>&1; then
    echo 'expected hash mismatch refusal' >&2
    exit 1
fi

if "$script" --rootfs "$rootfs" --sha256 "$sha" --source gnx-test-rootfs --version 0.0.0 >/dev/null 2>&1; then
    echo 'expected missing architecture refusal' >&2
    exit 1
fi

badroot=$work/badroot
mkdir -p "$badroot/etc" "$badroot/var/lib/dpkg"
printf 'ID=gnx-test\n' > "$badroot/etc/os-release"
printf 'amd64\n' > "$badroot/var/lib/dpkg/arch"
badtar=$work/bad.tar
tar -C "$badroot" -cf "$badtar" .
badsha=$(sha256sum "$badtar" | awk '{print $1}')
if "$script" --rootfs "$badtar" --sha256 "$badsha" --source gnx-test-rootfs --version 0.0.0 --arch amd64 >/dev/null 2>&1; then
    echo 'expected missing commands refusal' >&2
    exit 1
fi

echo 'rootfs-validation tests passed'
