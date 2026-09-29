#!/bin/sh
# Build an external, checksum-pinned operator payload; never installs or starts it.
set -eu
cd "$(dirname "$0")/../.."
[ "$(uname -s)" = Linux ] || { echo 'Linux build host required' >&2; exit 2; }
[ "$#" = 1 ] || [ "$#" = 2 ] || { echo 'Usage: build-voice.sh NEW_OUTPUT_DIRECTORY [LINUX_BINARY]' >&2; exit 2; }
out=$1
[ ! -e "$out" ] && [ ! -L "$out" ] || { echo 'New output directory required' >&2; exit 2; }
if [ "$#" = 2 ]; then
    binary=$2
else
    cargo build --locked --release --manifest-path ops/voice/Cargo.toml
    binary=ops/voice/target/release/gnx-voice-config
fi
[ "$(od -An -tx1 -N4 "$binary" | tr -d ' \n')" = 7f454c46 ] || { echo 'Linux ELF binary required' >&2; exit 2; }
umask 077
mkdir "$out"
mkdir "$out/payload"
cp "$binary" "$out/gnx-voice-config"
chmod 755 "$out/gnx-voice-config"
cp config/voice.toml "$out/voice.toml"
# Exact allowlist: never package .env files or runtime secrets.
for file in gnx-voice.pod gnx-voice-state.volume gnx-voice.container.in gnx-voice-access.container.in serve.json; do
    tr -d '\r' < "runtime/voice/$file" > "$out/payload/$file"
done
(
    cd "$out/payload"
    printf '{"schema":1,"files":{'
    comma=''
    for file in gnx-voice.pod gnx-voice-state.volume gnx-voice.container.in gnx-voice-access.container.in serve.json; do
        hash=$(sha256sum "$file" | cut -d ' ' -f 1)
        printf '%s"%s":"%s"' "$comma" "$file" "$hash"
        comma=','
    done
    printf '}}\n'
) > "$out/payload/manifest.json"
# Publish this non-secret hash through the reviewed release channel.
sha256sum "$out/payload/manifest.json"
printf '%s\n' 'BUILT_NOT_DEPLOYED: review images, guest, secrets and network policy before supply.'
