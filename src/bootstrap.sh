set -eu
. /etc/os-release
test "$ID" = ubuntu && test "$VERSION_ID" = 24.04
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y systemd systemd-sysv podman uidmap dbus-user-session slirp4netns fuse-overlayfs
id runtime >/dev/null 2>&1 || useradd --create-home --shell /bin/bash runtime
passwd --lock runtime
printf '[boot]\nsystemd=true\n[user]\ndefault=runtime\n[automount]\nenabled=false\n[interop]\nenabled=false\nappendWindowsPath=false\n' > /etc/wsl.conf
mkdir -p /var/lib/systemd/linger
touch /var/lib/systemd/linger/runtime
install -d -m 700 -o runtime -g runtime /home/runtime/.config /home/runtime/.config/containers /home/runtime/.config/containers/systemd
