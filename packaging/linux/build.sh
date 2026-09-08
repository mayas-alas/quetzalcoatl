#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p dist
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/usr/local/bin" "$stage/usr/local/share/gnx" "$stage/etc/gnx"
cp dist/gnx-linux "$stage/usr/local/bin/gnx"
chmod 755 "$stage/usr/local/bin/gnx"
cp -R runtime "$stage/usr/local/share/gnx/"
cp config/gnx.example.toml "$stage/etc/gnx/gnx.example.toml"
tar -C "$stage" -cf dist/gnx-linux-bundle.tar .
hash=$(sha256sum dist/gnx-linux-bundle.tar | cut -d ' ' -f1)
cat > dist/gnx-linux.run <<EOF
#!/bin/sh
set -eu
[ "\$(id -u)" = 0 ] || { echo 'Run as root' >&2; exit 2; }
umask 077
stage=\$(mktemp -d)
trap 'rm -rf "\$stage"' EXIT
tail -n +15 "\$0" > "\$stage/bundle.tar"
echo '$hash  '"\$stage/bundle.tar" | sha256sum -c - >&2
tar -xf "\$stage/bundle.tar" -C /
[ -e /etc/gnx/gnx.toml ] || cp /etc/gnx/gnx.example.toml /etc/gnx/gnx.toml
chmod 600 /etc/gnx/gnx.toml
printf '%s\n' 'Development core installed; run gnx doctor. Runtime capabilities are pending.' >&2
exit 0
# payload
EOF
cat dist/gnx-linux-bundle.tar >> dist/gnx-linux.run
chmod 755 dist/gnx-linux.run
