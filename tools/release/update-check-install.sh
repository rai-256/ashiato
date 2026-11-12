#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROGRAM='C:/Program Files/ashiato'
UPDATER="$PROGRAM/ashiato-updater.exe"
UPDATER_DIR='C:/ProgramData/ashiato-updater'
RULE='ashiato-update-check-ci-outbound'
URL='http://127.0.0.1:18701'
WORK=''
PORT_PID=''

die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 --block-outbound | --self-test | --list | <配布 zip>" >&2; exit 2; }

scenario_names=(
  '更新係はサービス専用のアカウントで自動で起きる'
  '更新係のアカウントは状態の置き場を読めない'
  '更新係の置き場は、サーバが読めて書けない'
  '切ると更新係のサービスが止まり、問い合わせが無くなる'
  '入れ直すと更新係が起きて確かめる'
  'インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる'
  '配布元に届かなくてもインストールは終わる'
  '更新係の無いインストールを走らせ直すと、更新係が足される'
  '切った更新の確認は、走らせ直しでも切ったまま'
  'repair は更新係に触れない'
)

list_scenarios() { printf '%s\n' "${scenario_names[@]}"; }

windows_dir() { cygpath -u "${WINDIR:-${SystemRoot:-C:\\Windows}}"; }
windows_netsh() { MSYS2_ARG_CONV_EXCL='*' "$(windows_dir)/System32/netsh.exe" "$@"; }
windows_sc() { MSYS2_ARG_CONV_EXCL='*' "$(windows_dir)/System32/sc.exe" "$@" 2>/dev/null | tr -d '\r'; }
windows_icacls() { MSYS2_ARG_CONV_EXCL='*' "$(windows_dir)/System32/icacls.exe" "$@" 2>/dev/null | tr -d '\r'; }

# 127.0.0.0/8 と ::1 は残す。インストールされた updater だけを対象にするため、
# PostgreSQL・サーバ・インストーラの loopback 通信は壊さない。
block_outbound() {
  windows_netsh advfirewall firewall add rule "name=$RULE" dir=out action=block \
    "program=$(win "$UPDATER")" "remoteip=0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff" enable=yes
}

win() { printf '%s' "${1//\//\\}"; }
state_code() { windows_sc query "$1" | awk -F ':' 'toupper($1) ~ /STATE/ { split($2,a," "); print a[1]; exit }'; }
wait_state() {
  local service="$1" expected="$2" limit="${3:-90}" deadline=$((SECONDS + limit))
  while [[ "$(state_code "$service")" != "$expected" ]]; do
    [[ $SECONDS -lt $deadline ]] || die "$service が状態 $expected にならない"
    sleep 1
  done
}
assert_service() {
  local service="$1" qc
  qc="$(windows_sc qc "$service")"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' <<<"$qc" || die "$service が自動で起きない"
  grep -Eiq "SERVICE_START_NAME[[:space:]]*:[[:space:]]*NT SERVICE\\\\$service[[:space:]]*$" <<<"$qc" ||
    die "$service が専用アカウントで動かない"
}
wait_updater_request() {
  local count_file="$1" deadline=$((SECONDS + 60)
  )
  while [[ ! -s "$count_file" ]]; do
    [[ $SECONDS -lt $deadline ]] || die '更新係の問い合わせが無い'
    sleep 1
  done
}

start_receiver() {
  local count_file="$1" port=18701
  python - "$port" "$count_file" <<'PY' &
import http.server, pathlib, sys
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        p = pathlib.Path(sys.argv[2])
        n = int(p.read_text() or '0') if p.exists() else 0
        p.write_text(str(n + 1))
        data = b'[]'
        self.send_response(200); self.send_header('Content-Length', str(len(data))); self.end_headers(); self.wfile.write(data)
    def log_message(self, *_): pass
http.server.HTTPServer(('127.0.0.1', int(sys.argv[1])), Handler).serve_forever()
PY
  PORT_PID=$!
}

stop_receiver() { [[ -z "$PORT_PID" ]] || { kill "$PORT_PID" 2>/dev/null || true; wait "$PORT_PID" 2>/dev/null || true; PORT_PID=''; }; }
cleanup() { local rc=$?; stop_receiver; [[ -z "$WORK" ]] || rm -rf "$WORK"; exit "$rc"; }

install_zip() {
  local archive="$1" output="$2" test_url=''
  unzip -q "$archive" -d "$WORK/unpacked"
  [[ -f "$WORK/unpacked/ashiato-setup.exe" ]] || die 'zip にインストーラが無い'
  if [[ "$archive" == *package-windows-test* ]]; then test_url="$URL"; fi
  if [[ -n "$test_url" ]]; then
    printf '%s\n%s\n' 'update-check-test-password-0123' 'update-check-test-password-0123' |
      env ASHIATO_SETUP_TEST_UPDATER_URL="$test_url" ASHIATO_SETUP_TEST_TRACE=1 \
      "$WORK/unpacked/ashiato-setup.exe" install >"$output" 2>&1
  else
    printf '%s\n%s\n' 'update-check-release-password-0123' 'update-check-release-password-0123' |
      "$WORK/unpacked/ashiato-setup.exe" install >"$output" 2>&1
  fi
}

check_zip() {
  local archive="$1" out
  [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] || die '実地の確かめは Windows（Git Bash）でだけ走る'
  export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' PYTHONUTF8=1 PYTHONIOENCODING=utf-8
  [[ -f "$archive" ]] || die "配布 zip が無い: $archive"
  WORK="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-check.XXXXXX")"
  trap cleanup EXIT
  out="$WORK/install.out"
  for service in ashiato-updater ashiato-server ashiato-db; do
    windows_sc query "$service" >/dev/null && { windows_sc stop "$service" >/dev/null || true; windows_sc delete "$service" >/dev/null || true; } || true
  done
  rm -rf "$PROGRAM" "$UPDATER_DIR"

  start_receiver "$WORK/requests"
  echo '=== 更新係（配る zip | 試験の zip）'
  install_zip "$archive" "$out" || { cat "$out" >&2; die '更新係を含むインストールが失敗'; }

  echo '== Scenario: 更新係はサービス専用のアカウントで自動で起きる'
  assert_service ashiato-updater
  [[ "$(state_code ashiato-updater)" == 4 ]] || die '更新係が実行中でない'

  echo '== Scenario: 更新係のアカウントは状態の置き場を読めない'
  ! windows_icacls 'C:\ProgramData\ashiato' /T | grep -Fqi 'NT SERVICE\\ashiato-updater' || die '状態の ACL に更新係がある'

  echo '== Scenario: 更新係の置き場は、サーバが読めて書けない'
  local updater_acl="$(windows_icacls "$(win "$UPDATER_DIR")" /T)"
  grep -Fqi 'NT SERVICE\\ashiato-updater' <<<"$updater_acl" || die '更新係の ACL にサービスが無い'
  ! grep -Eqi 'Users|Authenticated Users|Everyone' <<<"$updater_acl" || die '更新係の置き場に広い ACL がある'
  grep -Fqi 'NT SERVICE\\ashiato-server' <<<"$updater_acl" || die 'サーバの読み取り ACL が無い'

  echo '== Scenario: 切ると更新係のサービスが止まり、問い合わせが無くなる'
  local before=0; [[ -f "$WORK/requests" ]] && before="$(<"$WORK/requests")"
  "$PROGRAM/ashiato-setup.exe" update-check off >/dev/null 2>&1 || die 'update-check off が失敗'
  wait_state ashiato-updater 1
  sleep 3
  local after=0; [[ -f "$WORK/requests" ]] && after="$(<"$WORK/requests")"
  [[ "$after" == "$before" ]] || die '無効化後に問い合わせが増えた'

  echo '== Scenario: 入れ直すと更新係が起きて確かめる'
  "$PROGRAM/ashiato-setup.exe" update-check on >/dev/null 2>&1 || die 'update-check on が失敗'
  wait_state ashiato-updater 4
  if [[ "$archive" == *package-windows-test* ]]; then
    wait_updater_request "$WORK/requests"
  fi

  echo '== Scenario: インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる'
  grep -Fq '更新の確認: 有効' "$out" || die '有効の案内が無い'
  grep -Fq 'ashiato-setup.exe update-check off' "$out" || die '切り方の案内が無い'
  if [[ "$archive" == *package-windows-test* ]]; then
    local notice start
    notice="$(grep -n '更新の確認: 有効' "$out" | tail -n1 | cut -d: -f1)"
    start="$(grep -n 'result=ok sc.exe start ashiato-updater' "$out" | tail -n1 | cut -d: -f1)"
    [[ -n "$start" && "$start" -gt "$notice" ]] || die '案内より後に更新係を開始していない'
  fi

  echo '== Scenario: 配布元に届かなくてもインストールは終わる'
  [[ "$(state_code ashiato-db)" == 4 && "$(state_code ashiato-server)" == 4 ]] || die '本体のサービスが動いていない'
  [[ -f "$UPDATER_DIR/checks.jsonl" ]] || die '更新係の記録が無い'
  if [[ "$archive" != *package-windows-test* ]]; then
    grep -Eq 'unreachable|timeout' "$UPDATER_DIR/checks.jsonl" || die '配布元に届かない記録が無い'
  fi

  echo '== Scenario: 更新係の無いインストールを走らせ直すと、更新係が足される'
  windows_sc stop ashiato-updater >/dev/null || true; windows_sc delete ashiato-updater >/dev/null || true
  rm -rf "$UPDATER_DIR"
  "$PROGRAM/ashiato-setup.exe" install >/dev/null 2>&1 || die '走らせ直しが失敗'
  assert_service ashiato-updater
  [[ -d "$UPDATER_DIR" ]] || die '更新係の置き場ができない'

  echo '== Scenario: 切った更新の確認は、走らせ直しでも切ったまま'
  "$PROGRAM/ashiato-setup.exe" update-check off >/dev/null 2>&1 || die '再実行前の無効化が失敗'
  "$PROGRAM/ashiato-setup.exe" install >/dev/null 2>&1 || die '無効状態の走らせ直しが失敗'
  grep -Fq '"enabled":false' "$UPDATER_DIR/settings/check.json" || die '無効状態が保持されない'
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' <<<"$(windows_sc qc ashiato-updater)" || die '無効の開始種類が保持されない'

  echo '== Scenario: repair は更新係に触れない'
  local updater_sum="$(sha256sum "$UPDATER_DIR/checks.jsonl" | cut -d' ' -f1)"
  "$PROGRAM/ashiato-setup.exe" repair >/dev/null 2>&1 || die 'repair が失敗'
  [[ "$(sha256sum "$UPDATER_DIR/checks.jsonl" | cut -d' ' -f1)" == "$updater_sum" ]] || die 'repair が更新係の記録を変えた'
  echo 'OK: 更新係の Windows CI の実地の確かめ'
}

self_test() {
  local work fake log body name count fake_output
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-self-test.XXXXXX")"
  trap 'rm -rf "$work"' EXIT
  fake="$work/windows/System32"; mkdir -p "$fake"
  cat >"$fake/netsh.exe" <<'EOF'
#!/usr/bin/env bash
printf '<%s>\n' "$@"
EOF
  chmod +x "$fake/netsh.exe"
  cat >"$fake/sc.exe" <<'EOF'
#!/usr/bin/env bash
printf 'SERVICE_NAME: %s\n        START_TYPE         : 2   AUTO_START\n        SERVICE_START_NAME : NT SERVICE\\%s\n' "$2" "$2"
EOF
  chmod +x "$fake/sc.exe"
  cat >"$fake/icacls.exe" <<'EOF'
#!/usr/bin/env bash
printf 'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(OI)(CI)(M)\nC:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(RX)\n'
EOF
  chmod +x "$fake/icacls.exe"
  log="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" block_outbound)"
  grep -Fq '<dir=out>' <<<"$log" && grep -Fq '<action=block>' <<<"$log" || die 'netsh の遮断引数が違う'
  grep -Fq '<remoteip=0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff>' <<<"$log" || die 'loopback 以外の範囲で遮断していない'
  fake_output="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" windows_sc qc ashiato-updater)"
  grep -Fq 'START_TYPE         : 2' <<<"$fake_output" || die '偽の sc.exe を読めない'
  fake_output="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" windows_icacls 'C:\\ProgramData\\ashiato-updater' /T)"
  grep -Fqi 'ashiato-updater' <<<"$fake_output" || die '偽の icacls を読めない'
  body="$(awk '/^check_zip\(\)/,/^self_test\(\)/' "$0")"
  while IFS= read -r name; do count="$(grep -Fxc -- "  echo '== Scenario: $name'" <<<"$body" || true)"; [[ $count == 1 ]] || die "Scenario の印が不正: $name"; done < <(list_scenarios)
  ! awk '/^self_test\(\)/{on=1} on' "$0" | grep -v 'grep -Eq' | grep -Eq '^[[:space:]]*echo .== Scenario:' || die 'self-test に Scenario の印がある'
  rm -rf "$work"; trap - EXIT
  echo 'OK: update-check-install の self-test'
}

case "${1:-}" in
  --self-test) [[ $# == 1 ]] || usage; self_test ;;
  --list) [[ $# == 1 ]] || usage; list_scenarios ;;
  --block-outbound) [[ $# == 1 ]] || usage; block_outbound ;;
  '') usage ;;
  *) [[ $# == 1 ]] || usage; check_zip "$1" ;;
esac
