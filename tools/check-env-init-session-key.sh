#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/.env" <<'ENV'
API_TOKEN=keep-api-token
WEB_PASSWORD=keep-web-password
ENV
before="$(cat "$tmp/.env")"
ENV_INIT_ROOT="$tmp" ASHIATO_DB_PORT=55432 ASHIATO_HTTP_PORT=18787 bash "$root/tools/env-init.sh"
test "$(grep -c '^WEB_SESSION_KEY=' "$tmp/.env")" -eq 1
test "$(sed -n '1,2p' "$tmp/.env")" = "$before"
first="$(sed -n 's/^WEB_SESSION_KEY=//p' "$tmp/.env")"
ENV_INIT_ROOT="$tmp" ASHIATO_DB_PORT=55432 ASHIATO_HTTP_PORT=18787 bash "$root/tools/env-init.sh"
test "$(grep -c '^WEB_SESSION_KEY=' "$tmp/.env")" -eq 1
test "$(sed -n 's/^WEB_SESSION_KEY=//p' "$tmp/.env")" = "$first"
echo "PASS: WEB_SESSION_KEY を一度だけ追加し、既存行を保持した"
