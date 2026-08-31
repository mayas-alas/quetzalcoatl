#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
distribution_directory="${repository_root}/dist"
appdir="${repository_root}/target/appimage/GNX.AppDir"
tools_directory="${repository_root}/target/appimage-tools"
linux_binary="${repository_root}/target/linux-musl/release/gnx"
output="${distribution_directory}/gnx-x86_64.AppImage"
builder_image="docker.io/library/rust@sha256:3ffeca71d0e4fc30f5537f76b7243e87ac99726b6d3d66591dfc5e497078b9fc"

lock_value() {
  local section="$1"
  local key="$2"
  awk -v wanted="[${section}]" -v key="${key}" '
    $0 == wanted { active=1; next }
    active && /^\[/ { exit }
    active && $1 == key { value=$3; gsub(/"/, "", value); print value; exit }
  ' "${repository_root}/dependencies.lock.toml"
}

download_verified() {
  local url="$1"
  local sha256="$2"
  local destination="$3"
  if [[ -f "${destination}" ]] && echo "${sha256}  ${destination}" | sha256sum --check --status; then
    return
  fi
  rm -f "${destination}.download"
  curl --fail --location --proto '=https' --tlsv1.2 --output "${destination}.download" "${url}"
  echo "${sha256}  ${destination}.download" | sha256sum --check
  mv "${destination}.download" "${destination}"
}

if [[ "$(uname -m)" != "x86_64" ]]; then
  echo "GNX AppImage sólo soporta Linux x86_64." >&2
  exit 7
fi

mkdir -p "${distribution_directory}" "${tools_directory}"

if [[ "${1:-}" != "--skip-compile" ]]; then
  podman run --rm --arch amd64 \
    --volume "${repository_root}:/workspace" \
    --workdir /workspace \
    --env CARGO_TARGET_DIR=/workspace/target/linux-musl \
    "${builder_image}" \
    sh -c 'cargo test --locked --all-targets && cargo build --locked --release'
fi

if [[ ! -x "${linux_binary}" ]]; then
  echo "Falta ${linux_binary}; ejecute el build Linux primero." >&2
  exit 2
fi

rm -rf "${appdir}"
install -d "${appdir}/usr/bin"
install -d "${appdir}/usr/share/metainfo"
install -m 0755 "${linux_binary}" "${appdir}/usr/bin/gnx"
install -m 0755 "${repository_root}/packaging/appimage/AppRun" "${appdir}/AppRun"
install -m 0644 "${repository_root}/packaging/appimage/gnx.desktop" "${appdir}/gnx.desktop"
install -m 0644 "${repository_root}/packaging/appimage/gnx.svg" "${appdir}/gnx.svg"
install -m 0644 "${repository_root}/packaging/appimage/gnx.metainfo.xml" \
  "${appdir}/usr/share/metainfo/org.gnx.QuetzalcoatlNext.metainfo.xml"

appimagetool_url="$(lock_value build.appimagetool url)"
appimagetool_sha256="$(lock_value build.appimagetool sha256)"
runtime_url="$(lock_value build.appimage_runtime url)"
runtime_sha256="$(lock_value build.appimage_runtime sha256)"
appimagetool="${tools_directory}/appimagetool-x86_64.AppImage"
runtime="${tools_directory}/runtime-x86_64"

download_verified "${appimagetool_url}" "${appimagetool_sha256}" "${appimagetool}"
download_verified "${runtime_url}" "${runtime_sha256}" "${runtime}"
chmod 0755 "${appimagetool}"

tool_extract_directory="${tools_directory}/appimagetool-extracted"
rm -rf "${tool_extract_directory}"
mkdir -p "${tool_extract_directory}"
(
  cd "${tool_extract_directory}"
  "${appimagetool}" --appimage-extract >/dev/null
)

version="$(cargo metadata --locked --no-deps --format-version 1 | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -n 1)"
rm -f "${output}"
ARCH=x86_64 VERSION="${version}" \
  "${tool_extract_directory}/squashfs-root/AppRun" \
  --runtime-file "${runtime}" "${appdir}" "${output}"
chmod 0755 "${output}"

"${output}" --appimage-extract-and-run version
sha256sum "${output}" | sed 's#  .*/# *#' >"${distribution_directory}/SHA256SUMS.appimage"
echo "Creado: ${output}"
