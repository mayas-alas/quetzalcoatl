set -eu
. /etc/os-release
test "$ID" = ubuntu && test "$VERSION_ID" = 24.04
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y systemd systemd-sysv podman uidmap dbus-user-session slirp4netns fuse-overlayfs
id @LINUX_USER@ >/dev/null 2>&1 || useradd --create-home --shell /bin/bash @LINUX_USER@
passwd --lock @LINUX_USER@
printf '[boot]\nsystemd=true\n[user]\ndefault=@LINUX_USER@\n[automount]\nenabled=false\n[interop]\nenabled=false\nappendWindowsPath=false\n' > /etc/wsl.conf
mkdir -p /var/lib/systemd/linger
touch /var/lib/systemd/linger/@LINUX_USER@
install -d -m 700 -o @LINUX_USER@ -g @LINUX_USER@ /home/@LINUX_USER@/.config /home/@LINUX_USER@/.config/containers /home/@LINUX_USER@/.config/containers/systemd
