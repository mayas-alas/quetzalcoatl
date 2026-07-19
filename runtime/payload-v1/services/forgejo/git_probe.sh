#!/bin/sh
set -eu
umask 077
export LC_ALL=C

password_file='/run/secrets/forgejo/admin_password'
askpass='/tmp/gnx-i1-git-askpass'
work='/tmp/gnx-i1-git-evidence'

cleanup() {
  rm -f "$askpass"
  rm -rf "$work"
}
trap cleanup EXIT HUP INT TERM

test "$(id -u)" = 1000
test "$(stat -c '%u:%g:%a' "$password_file")" = '1000:1000:400'
command -v git >/dev/null

printf '%s\n' \
  '#!/bin/sh' \
  'case "${1:-}" in' \
  '  *Username*) printf "%s\\n" gnx-admin ;;' \
  '  *Password*) cat /run/secrets/forgejo/admin_password ;;' \
  '  *) exit 1 ;;' \
  'esac' > "$askpass"
chmod 0700 "$askpass"

rm -rf "$work"
mkdir -p "$work/source"
chmod 0700 "$work" "$work/source"
git -C "$work/source" init -q --initial-branch=main
git -C "$work/source" config user.name 'Quetzalcoatl I1'
git -C "$work/source" config user.email 'gnx-admin@example.invalid'
evidence_value=$(cat /proc/sys/kernel/random/uuid)
printf '%s\n' "$evidence_value" > "$work/source/i1-evidence.txt"
git -C "$work/source" add i1-evidence.txt
git -C "$work/source" commit -q -m 'I1 functional evidence'

git_url='http://127.0.0.1:3000/gnx-admin/i1-evidence.git'
GIT_ASKPASS="$askpass" GIT_TERMINAL_PROMPT=0 \
  git -C "$work/source" push -q --force "$git_url" main
GIT_ASKPASS="$askpass" GIT_TERMINAL_PROMPT=0 \
  git clone -q --branch main --single-branch "$git_url" "$work/clone"
test "$(cat "$work/clone/i1-evidence.txt")" = "$evidence_value"
source_commit=$(git -C "$work/source" rev-parse HEAD)
test "$source_commit" = "$(git -C "$work/clone" rev-parse HEAD)"
printf '%s\n' "$source_commit" | grep -Eq '^[0-9a-f]{40}$'
printf 'FORGEJO_PUSH_CLONE=ready;COMMIT=%s\n' "$source_commit"
