#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tools_directory="${repository_root}/target/opentofu-tools"
archive="${tools_directory}/tofu.tar.gz"

lock_value() {
  local section="$1"
  local key="$2"
  awk -v wanted="[${section}]" -v key="${key}" '
    $0 == wanted { active=1; next }
    active && /^\[/ { exit }
    active && $1 == key { value=$3; gsub(/"/, "", value); print value; exit }
  ' "${repository_root}/dependencies.lock.toml"
}

url="$(lock_value runtime.opentofu url)"
sha256="$(lock_value runtime.opentofu sha256)"
mkdir -p "${tools_directory}"
if [[ ! -f "${archive}" ]] || ! echo "${sha256}  ${archive}" | sha256sum --check --status; then
  curl --fail --location --proto '=https' --tlsv1.2 --output "${archive}.download" "${url}"
  echo "${sha256}  ${archive}.download" | sha256sum --check
  mv "${archive}.download" "${archive}"
fi
tar -xzf "${archive}" -C "${tools_directory}" tofu
"${tools_directory}/tofu" -chdir="${repository_root}/infra/opentofu" providers lock -platform=linux_amd64
