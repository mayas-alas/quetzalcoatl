# Executed inside the access container; credential arrives only through stdin.
set -eu
umask 077
key=$(mktemp /run/gnx/enrollment.XXXXXX)
trap 'rm -f -- "$key"' EXIT
trap 'exit 130' HUP INT TERM
cat > "$key"
result=0
"$@" --auth-key="file:$key" || result=$?
rm -f -- "$key"
trap - EXIT
exit "$result"
