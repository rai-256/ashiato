#!/usr/bin/env bash
# 設定ファイルを読むサービス形のサーバを、Linux の試験用に起こす。
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$PWD"
PID_FILE="${ASHIATO_SETTINGS_SERVER_PID_FILE:-$ROOT/.settings-server.pid}"

usage() {
  echo "使い方: $0 make <置き場> | $0 up <設定のファイル> | $0 down [設定のファイル] | $0 --self-test" >&2
  exit 2
}

load_dotenv() {
  if [[ -f .env ]]; then
    set -a
    # shellcheck disable=SC1091
    . ./.env
    set +a
  fi
}

settings_pid_file() {
  if [[ -n "${2:-}" ]]; then
    printf '%s.pid' "$2"
  else
    printf '%s' "$PID_FILE"
  fi
}

make_settings() {
  local place="${1:?置き場が必要です}" settings password_file
  place="$(cd "$place" 2>/dev/null && pwd || { mkdir -p "$place"; cd "$place"; pwd; })"
  settings="$place/settings/server.json"
  password_file="$place/web-password"
  mkdir -p "${settings%/*}"

  . tools/ports.sh
  local database_url="${DATABASE_URL:-postgres://ashiato_app:st59-test-db@127.0.0.1:${ASHIATO_DB_PORT}/ashiato}"
  database_url="$(ashiato_db_url "$database_url")"
  local bind="127.0.0.1:$((ASHIATO_HTTP_PORT + 1))"
  local api_token='st59-fake-api-token-012345'
  local session_key='st59-fake-session-key-012345'
  # 固定の試験用合言葉に対応する Argon2id の PHC 文字列。値は端末へ出さない。
  local verifier='$argon2id$v=19$m=19456,t=2,p=1$gx8nAvrN+FgcZDnoorN72w$hiiidQ2czwjXOH2yOteA3cHkda6rm/ihXSebziLpfvk'
  local password='st59-test-password-012345'

  umask 077
  printf '{"API_TOKEN":"%s","BIND":"%s","DATABASE_URL":"%s","WEB_PASSWORD_VERIFIER":"%s","WEB_SESSION_KEY":"%s"}\n' \
    "$api_token" "$bind" "$database_url" "$verifier" "$session_key" >"$settings"
  printf '%s\n' "$password" >"$password_file"
  echo "設定を作った（鍵は表示しない）: $settings"
}

apply_settings_to_environment() {
  local settings="${1:?設定のファイルが必要です}" key value
  [[ -f "$settings" ]] || { echo "error: 設定のファイルが無い" >&2; return 1; }
  unset WEB_PASSWORD
  while IFS=$'\t' read -r key value; do
    [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "error: 設定の鍵が不正" >&2; return 1; }
    export "$key=$value"
  done < <(jq -r 'to_entries[] | [.key, (.value | strings)] | @tsv' "$settings")
  export BIND="127.0.0.1:$((ASHIATO_HTTP_PORT + 1))"
  export ASHIATO_SERVICE_MODE=1
  export ASHIATO_SETTINGS_PATH="$settings"
}

build_server_if_needed() {
  local server="${SERVER_BIN:-target/release/ashiato-server}" web="${WEB_DIST:-web/dist}"
  if [[ ! -f "$web/index.html" && -z "${WEB_DIST:-}" ]]; then
    (cd web && npm ci --silent >/dev/null && npm run build --silent >/dev/null)
    web="web/dist"
  fi
  if [[ ! -x "$server" ]]; then
    local web_abs
    web_abs="$(cd "$web" && pwd)"
    ASHIATO_WEB_DIST="$web_abs" cargo build -q --release -p ashiato-server --bin ashiato-server
  fi
  printf '%s\n' "$server"
}

up_server() {
  local settings="${1:?設定のファイルが必要です}" server bind pid_file
  load_dotenv
  . tools/ports.sh
  apply_settings_to_environment "$settings"
  server="$(build_server_if_needed)"
  bind="$BIND"
  pid_file="$(settings_pid_file up "$settings")"
  [[ ! -e "$pid_file" ]] || { echo "error: settings-server は既に起動している" >&2; return 1; }
  # D8: 元の合言葉は親環境からもサーバへ渡さない。
  env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL -u WEB_PASSWORD \
    "$server" >"${pid_file}.log" 2>&1 &
  echo $! >"$pid_file"
  trap '[[ -f "$pid_file" ]] && kill "$(cat "$pid_file")" 2>/dev/null || true' EXIT INT TERM
  for _ in $(seq 1 30); do
    curl -sf "http://$bind/healthz" >/dev/null && break
    sleep 1
  done
  curl -sf "http://$bind/healthz" >/dev/null || { echo "error: サーバが起動しない" >&2; return 1; }
  echo "サーバを起こした: $bind"
  wait "$(cat "$pid_file")"
}

down_server() {
  local pid_file pid
  pid_file="$(settings_pid_file down "${1:-}")"
  [[ -f "$pid_file" ]] || { echo "サーバは起動していない"; return 0; }
  pid="$(cat "$pid_file")"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  rm -f "$pid_file" "${pid_file}.log"
  echo "サーバを止めた"
}

self_test() {
  local settings keys names
  self_test_tmp="$(mktemp -d)"
  trap 'rm -rf "${self_test_tmp:-}"' EXIT
  make_settings "$self_test_tmp" >/dev/null
  settings="$self_test_tmp/settings/server.json"
  keys="$(jq -r 'keys | join(" ")' "$settings")"
  [[ "$keys" == "API_TOKEN BIND DATABASE_URL WEB_PASSWORD_VERIFIER WEB_SESSION_KEY" ]] || return 1
  [[ -s "$self_test_tmp/web-password" ]] || return 1
  load_dotenv
  . tools/ports.sh
  apply_settings_to_environment "$settings"
  names="$(env -0 | tr '\0' '\n' | cut -d= -f1)"
  grep -qxF ASHIATO_SERVICE_MODE <<<"$names"
  grep -qxF ASHIATO_SETTINGS_PATH <<<"$names"
  ! grep -qxF WEB_PASSWORD <<<"$names"
  echo "self-test 通過（設定の鍵と起動環境の名前を確認した）"
}

case "${1:-}" in
  --self-test) self_test ;;
  make) make_settings "${2:-}" ;;
  up) up_server "${2:-}" ;;
  down) down_server "${2:-}" ;;
  *) usage ;;
esac
