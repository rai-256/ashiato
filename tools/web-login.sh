#!/usr/bin/env bash
set -euo pipefail

login() {
  local base=${1:?base URL が必要です} payload headers status set_cookie
  payload=$(mktemp)
  headers=$(mktemp)
  jq -nc --arg password "${WEB_PASSWORD:?WEB_PASSWORD が必要です}" '{password:$password}' >"$payload"
  if ! status=$(curl -sS -D "$headers" -o /dev/null -w '%{http_code}' \
    -H 'content-type: application/json' -X POST "$base/session" -d @"$payload"); then
    rm -f "$payload" "$headers"
    echo "web-login: $base/session に繋がらない" >&2
    return 1
  fi
  if [ "$status" != 204 ]; then
    rm -f "$payload" "$headers"
    # 理由は応答の番号だけ（合言葉も印も出さない）。401 = WEB_PASSWORD がサーバと違う・429 = 試行の上限
    echo "web-login: POST /session が $status（204 でない）" >&2
    return 1
  fi
  set_cookie=$(sed -n 's/^[Ss]et-[Cc]ookie: \([^;[:space:]]*\).*/\1/p' "$headers" | head -n 1)
  if [ -z "$set_cookie" ]; then
    rm -f "$payload" "$headers"
    echo "web-login: POST /session の応答に印（Set-Cookie）が無い" >&2
    return 1
  fi
  rm -f "$payload" "$headers"
  printf 'cookie: %s\n' "$set_cookie"
}

self_test() {
  local port good
  self_test_root=$(mktemp -d)
  self_test_pid=''
  cleanup() {
    [ -z "$self_test_pid" ] || kill "$self_test_pid" 2>/dev/null || true
    [ -z "$self_test_root" ] || rm -rf "$self_test_root"
  }
  trap cleanup EXIT
  python3 - "$self_test_root/port" >"$self_test_root/server.log" 2>&1 <<'PY' &
import http.server, pathlib, sys
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get("content-length", "0")))
        status = 401 if self.path == "/bad/session" else 204
        self.send_response(status)
        if status == 204:
            self.send_header("Set-Cookie", "self_test_session=opaque-value; Secure; HttpOnly")
        self.end_headers()
    def log_message(self, *_): pass
server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
pathlib.Path(sys.argv[1]).write_text(str(server.server_port))
server.serve_forever()
PY
  self_test_pid=$!
  for _ in $(seq 1 100); do [ -s "$self_test_root/port" ] && break; sleep 0.01; done
  port=$(cat "$self_test_root/port")
  good=$(WEB_PASSWORD='self-test-password' login "http://127.0.0.1:$port")
  if [ "$good" != 'cookie: self_test_session=opaque-value' ]; then return 1; fi
  if WEB_PASSWORD='self-test-password' login "http://127.0.0.1:$port/bad" >/dev/null 2>"$self_test_root/err"; then return 1; fi
  # 失敗は理由を出す（ST44 /code-review R22）。合言葉の値は出さない
  grep -q 'POST /session が 401' "$self_test_root/err" || return 1
  if grep -q 'self-test-password' "$self_test_root/err"; then return 1; fi
}

if [ "${1:-}" = --self-test ]; then
  self_test
else
  login "${1:?使い方: tools/web-login.sh <base_url>}"
fi
