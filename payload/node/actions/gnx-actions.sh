#!/usr/bin/env bash
# GnX private-repository GitHub Actions adapter.
# Credentials are accepted only from a protected file or stdin; never argv/env.
set -Eeuo pipefail
IFS=$'\n\t'
umask 077

usage() {
  cat <<'USAGE'
usage: gnx-actions.sh --config FILE [--token-file FILE | --token-stdin] dispatch|cancel|status [options]

operations:
  dispatch  Start the configured workflow (optional: --lease-minutes N --request-id ID --ref REF)
  cancel    Cancel a run (required: --run-id ID)
  status    Read one run, or the latest dispatch run (optional: --run-id ID)
USAGE
}
die() {
  local code=$1 message=$2
  json_error "$code" "$message"
  exit 64
}
json_error() {
  local code=$1 message=$2
  if python3 -c 'pass' >/dev/null 2>&1; then
    python3 -c 'import json,sys; print(json.dumps({"ok":False,"error":sys.argv[1],"message":sys.argv[2]}, separators=(",", ":")))' "$code" "$message"
  else
    code=${code//\\/\\\\}; code=${code//\"/\\\"}; message=${message//\\/\\\\}; message=${message//\"/\\\"}
    printf '{"ok":false,"error":"%s","message":"%s"}\n' "$code" "$message"
  fi
}

config_file=''
token_file=''
token_stdin=0
operation=''
run_id=''
lease_minutes=''
request_id=''
ref=''

# Parse options without eval/source. Values are validated after config loading.
while (($#)); do
  case "$1" in
    --config) (($# >= 2)) || die INVALID_ARGUMENT '--config requires a file'; config_file=$2; shift 2 ;;
    --token-file) (($# >= 2)) || die INVALID_ARGUMENT '--token-file requires a file'; token_file=$2; shift 2 ;;
    --token-stdin) ((token_stdin == 0 && -z "$token_file")) || die INVALID_ARGUMENT 'token source specified more than once'; token_stdin=1; shift ;;
    --run-id) (($# >= 2)) || die INVALID_ARGUMENT '--run-id requires a number'; run_id=$2; shift 2 ;;
    --lease-minutes) (($# >= 2)) || die INVALID_ARGUMENT '--lease-minutes requires a number'; lease_minutes=$2; shift 2 ;;
    --request-id) (($# >= 2)) || die INVALID_ARGUMENT '--request-id requires an id'; request_id=$2; shift 2 ;;
    --ref) (($# >= 2)) || die INVALID_ARGUMENT '--ref requires a git ref'; ref=$2; shift 2 ;;
    dispatch|cancel|status) [[ -z "$operation" ]] || die INVALID_ARGUMENT 'only one operation is allowed'; operation=$1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die INVALID_ARGUMENT "unknown argument: $1" ;;
  esac
done
[[ -n "$config_file" ]] || die CONFIG_REQUIRED '--config is required'
[[ -n "$operation" ]] || die OPERATION_REQUIRED 'dispatch, cancel, or status is required'
[[ -f "$config_file" && ! -L "$config_file" && -r "$config_file" ]] || die CONFIG_UNREADABLE 'config must be a readable regular file'
python3 -c 'import json' >/dev/null 2>&1 || die MISSING_PYTHON 'python3 with the json module is required'

# Config is deliberately a small, non-secret allowlist. Never source it as shell.
api_base='https://api.github.com'
repository=''
workflow=''
config_ref='main'
config_lease='60'
poll_seconds='10'
while IFS='=' read -r key value || [[ -n "$key" ]]; do
  [[ -z "$key" || "$key" == \#* ]] && continue
  [[ "$key" =~ ^GNX_[A-Z0-9_]+$ && "$value" != *$'\r'* && "$value" != *$'\n'* ]] || die CONFIG_INVALID 'invalid config line'
  case "$key" in
    GNX_GITHUB_API) api_base=$value ;;
    GNX_GITHUB_REPOSITORY) repository=$value ;;
    GNX_GITHUB_WORKFLOW) workflow=$value ;;
    GNX_GITHUB_REF) config_ref=$value ;;
    GNX_LEASE_MINUTES) config_lease=$value ;;
    GNX_POLL_SECONDS) poll_seconds=$value ;;
    GNX_GITHUB_TOKEN_FILE) [[ -z "$token_file" ]] && token_file=$value ;;
    *) die CONFIG_INVALID "unsupported config key: $key" ;;
  esac
done < "$config_file"

[[ "$api_base" =~ ^https://[^[:space:]/]+(/[^[:space:]]*)?$ ]] || die CONFIG_INVALID 'API must use HTTPS'
api_base=${api_base%/}
[[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die CONFIG_INVALID 'repository must be owner/name'
[[ "$workflow" =~ ^[A-Za-z0-9_.-]+$ ]] || die CONFIG_INVALID 'workflow must be a file name'
[[ "$config_ref" =~ ^[A-Za-z0-9._/@-]+$ ]] || die CONFIG_INVALID 'ref contains unsafe characters'
[[ "$config_lease" =~ ^[0-9]+$ && "$config_lease" -ge 1 && "$config_lease" -le 350 ]] || die CONFIG_INVALID 'lease must be 1..350 minutes'
[[ "$poll_seconds" =~ ^[0-9]+$ && "$poll_seconds" -ge 1 && "$poll_seconds" -le 300 ]] || die CONFIG_INVALID 'poll interval must be 1..300 seconds'
[[ -z "$lease_minutes" ]] && lease_minutes=$config_lease
[[ -z "$ref" ]] && ref=$config_ref
[[ "$lease_minutes" =~ ^[0-9]+$ && "$lease_minutes" -ge 1 && "$lease_minutes" -le 350 ]] || die INVALID_ARGUMENT 'lease must be 1..350 minutes'
[[ "$ref" =~ ^[A-Za-z0-9._/@-]+$ ]] || die INVALID_ARGUMENT 'ref contains unsafe characters'
[[ "$run_id" =~ ^[0-9]+$ || -z "$run_id" ]] || die INVALID_ARGUMENT 'run id must be numeric'
[[ "$request_id" =~ ^[A-Za-z0-9._-]{1,96}$ || -z "$request_id" ]] || die INVALID_ARGUMENT 'request id contains unsafe characters'
[[ "$operation" != cancel || -n "$run_id" ]] || die INVALID_ARGUMENT 'cancel requires --run-id'

read_token_file() {
  local path=$1 mode owner extra
  [[ -f "$path" && ! -L "$path" && -r "$path" ]] || die TOKEN_UNREADABLE 'token must be a readable regular file'
  mode=$(stat -c '%a' -- "$path") || die TOKEN_UNREADABLE 'cannot inspect token permissions'
  [[ "$mode" == 400 || "$mode" == 600 ]] || die TOKEN_PERMISSIONS 'token file must be mode 0400 or 0600'
  owner=$(stat -c '%u' -- "$path") || die TOKEN_UNREADABLE 'cannot inspect token owner'
  [[ "$owner" == "$(id -u)" || "$owner" == 0 ]] || die TOKEN_OWNER 'token owner is not this user or root'
  IFS= read -r token < "$path" || true
  if IFS= read -r extra < "$path"; then die TOKEN_FORMAT 'token file must contain one line'; fi
}
read_token_stdin() {
  IFS= read -r token || true
  if IFS= read -r extra; then die TOKEN_FORMAT 'token stdin must contain one line'; fi
}
token=''
extra=''
if ((token_stdin)); then
  read_token_stdin
else
  [[ -n "$token_file" ]] || die TOKEN_REQUIRED 'use --token-file or --token-stdin'
  read_token_file "$token_file"
fi
[[ "$token" =~ ^[A-Za-z0-9_.-]{20,}$ ]] || die TOKEN_FORMAT 'token has an invalid format'
unset extra

# Keep the Authorization value on curl's stdin. It is never a process argument.
api_request() {
  local method=$1 path=$2 body=${3-} response status
  if [[ -n "$body" ]]; then
    if ! response=$(printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\n' "$token" |
      curl --silent --show-error --location --fail-with-body --retry 2 --retry-all-errors --connect-timeout 15 --max-time 45 \
        --request "$method" --header @- --header 'Content-Type: application/json' --data-binary "$body" \
        --write-out $'\n%{http_code}' "$api_base$path" 2>/dev/null); then
      :
    fi
  else
    if ! response=$(printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\n' "$token" |
      curl --silent --show-error --location --fail-with-body --retry 2 --retry-all-errors --connect-timeout 15 --max-time 45 \
        --request "$method" --header @- --write-out $'\n%{http_code}' "$api_base$path" 2>/dev/null); then
      :
    fi
  fi
  if [[ "$response" =~ $'\n'([0-9]{3})$ ]]; then
    status=${BASH_REMATCH[1]}
    response=${response%$'\n'*}
  else
    status=000
  fi
  API_STATUS=$status
  API_BODY=$response
}

emit_response() {
  local status=$1 body=$2 operation_name=$3
  printf '%s' "$body" | python3 -c '
import json, sys
operation, repository, workflow, status = sys.argv[1:]
try:
    response = json.load(sys.stdin)
except (ValueError, TypeError):
    print(json.dumps({"ok": False, "operation": operation, "repository": repository,
                      "workflow": workflow, "http_status": None if status == "000" else int(status),
                      "error": "invalid_or_unavailable_response"}, separators=(",", ":")))
    raise SystemExit(0)
http_status = int(status)
ok = 200 <= http_status < 300
print(json.dumps({"ok": ok, "operation": operation, "repository": repository,
                  "workflow": workflow, "http_status": http_status, "response": response}, separators=(",", ":")))
' "$operation_name" "$repository" "$workflow" "$status" 2>/dev/null \
    || json_error API_RESPONSE 'GitHub returned no JSON response'
}

case "$operation" in
  dispatch)
    [[ -n "$request_id" ]] || request_id="gnx-${RANDOM}-${RANDOM}"
    payload=$(python3 -c 'import json,sys; print(json.dumps({"ref":sys.argv[1],"inputs":{"lease_minutes":sys.argv[2],"request_id":sys.argv[3]}}, separators=(",", ":")))' \
      "$ref" "$lease_minutes" "$request_id")
    api_request POST "/repos/$repository/actions/workflows/$workflow/dispatches" "$payload"
    if [[ "$API_STATUS" == 204 ]]; then
      python3 -c 'import json,sys; print(json.dumps({"ok":True,"operation":"dispatch","repository":sys.argv[1],"workflow":sys.argv[2],"ref":sys.argv[3],"lease_minutes":int(sys.argv[4]),"request_id":sys.argv[5],"http_status":204}, separators=(",", ":")))' \
        "$repository" "$workflow" "$ref" "$lease_minutes" "$request_id"
    else
      emit_response "$API_STATUS" "$API_BODY" dispatch
      exit 1
    fi
    ;;
  cancel)
    api_request POST "/repos/$repository/actions/runs/$run_id/cancel"
    if [[ "$API_STATUS" == 202 || "$API_STATUS" == 204 ]]; then
      python3 -c 'import json,sys; print(json.dumps({"ok":True,"operation":"cancel","repository":sys.argv[1],"workflow":sys.argv[2],"run_id":int(sys.argv[3]),"http_status":int(sys.argv[4])}, separators=(",", ":")))' \
        "$repository" "$workflow" "$run_id" "$API_STATUS"
    else
      emit_response "$API_STATUS" "$API_BODY" cancel
      exit 1
    fi
    ;;
  status)
    if [[ -n "$run_id" ]]; then
      api_request GET "/repos/$repository/actions/runs/$run_id"
    else
      api_request GET "/repos/$repository/actions/workflows/$workflow/runs?event=workflow_dispatch&per_page=20"
    fi
    if [[ "$API_STATUS" =~ ^2[0-9][0-9]$ ]]; then
      emit_response "$API_STATUS" "$API_BODY" status
    else
      emit_response "$API_STATUS" "$API_BODY" status
      exit 1
    fi
    ;;
esac
