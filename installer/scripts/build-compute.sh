#!/usr/bin/env bash
# Release image builder; never execute in the enrolled node identity.
set -Eeuo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
source_dir="$root/payload/node/compute/image"
output="$root/build/compute"
for tool in podman skopeo sha256sum python3; do command -v "$tool" >/dev/null; done
[[ $(uname -m) == x86_64 ]] || { printf 'amd64 builder required\n' >&2; exit 1; }
mkdir -p "$output"
# A build tag is not the runtime reference. Promotion records the exported digest.
image=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["build_reference"])' "$source_dir/release.json")
[[ "$image" =~ ^localhost/gnx/proxmox:[a-zA-Z0-9_.-]+$ ]] || { printf 'Invalid build reference\n' >&2; exit 1; }
podman build --network=host --format=oci --build-arg "VERSION_ARG=${image##*:}" --tag "$image" --file "$source_dir/Containerfile" "$source_dir"
podman run --rm --network=none --entrypoint /bin/bash "$image" -c '
set -Eeuo pipefail
! dpkg-query -W -f="\${db:Status-Status}\n" pve-fake-subscription 2>/dev/null | grep -qx installed
test ! -e /tmp/sub.deb
bash -n /usr/local/bin/entrypoint.sh /usr/local/bin/network.sh
dpkg-query -W -f="\${binary:Package}\t\${Version}\n"
' > "$output/packages.tsv"
skopeo copy --format oci "containers-storage:$image" "oci-archive:$output/proxmox.oci.tar:$image"
sha256sum "$output/proxmox.oci.tar" | awk '{print $1 "  proxmox.oci.tar"}' > "$output/SHA256SUMS.txt"
skopeo inspect --raw "oci-archive:$output/proxmox.oci.tar" | sha256sum | awk '{print "sha256:" $1}' > "$output/manifest.digest"
digest=$(< "$output/manifest.digest")
# Archive export can compress layers and change the manifest digest. Promote the
# exported manifest, never the build image ID or the uncompressed storage digest.
skopeo copy --preserve-digests "oci-archive:$output/proxmox.oci.tar" "containers-storage:$image"
podman image exists "localhost/gnx/proxmox@$digest"
skopeo inspect --config "oci-archive:$output/proxmox.oci.tar" | python3 -c '
import json,sys
image=json.load(sys.stdin)
assert image["architecture"] == "amd64" and image["os"] == "linux"
assert image["config"]["Entrypoint"] == ["/usr/local/bin/entrypoint.sh"]
assert image["config"]["Cmd"] == ["/sbin/init", "--log-target=console", "--log-level=notice"]
'
# Never update a production Quadlet as a side effect of a build.
printf 'Validated OCI: %s\nRuntime digest: %s\n' "$output/proxmox.oci.tar" "$digest"
