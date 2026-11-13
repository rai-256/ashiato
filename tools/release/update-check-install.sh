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
  local count_file="$1" expected="$2" deadline=$((SECONDS + 60)) count
  while :; do
    count="$(<"$count_file" 2>/dev/null || printf 0)"
    [[ "$count" -ge "$expected" ]] && return 0
    [[ $SECONDS -lt $deadline ]] || die '更新係の問い合わせが無い'
    sleep 1
  done
}

assert_disabled_service() {
  local service="$1" qc
  qc="$(windows_sc qc "$service")"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' <<<"$qc" || die "$service が無効でない"
}

assert_disabled_setting() {
  local path="$1"
  grep -Eq '"enabled"[[:space:]]*:[[:space:]]*false' "$path" || die '無効状態が保持されない'
}

assert_updater_acl_text() {
  local acl="$1" check_acl
  ! grep -Eq '^.*\(I\)' <<<"$acl" || die '更新係の ACL が親から継承されている'
  grep -Fq 'NT SERVICE\ashiato-updater:(OI)(CI)(M)' <<<"$acl" || die '更新係の書き ACL が無い'
  grep -Fq 'NT SERVICE\ashiato-server:(OI)(CI)(RX)' <<<"$acl" || die 'サーバの読み取り ACL が無い'
  ! grep -Eqi 'Users|Authenticated Users|Everyone' <<<"$acl" || die '更新係の置き場に広い ACL がある'
  check_acl="$(windows_icacls "$(win "$UPDATER_DIR")\settings\check.json")"
  ! grep -Eqi 'NT SERVICE\\(ashiato-updater|ashiato-server).*\((F|M)\)' <<<"$check_acl" || die 'check.json にサービスの書き ACL がある'
  grep -Eqi 'NT SERVICE\\(ashiato-updater|ashiato-server).*\(R\)' <<<"$check_acl" || die 'check.json のサービス読み取り ACL が無い'
  grep -Eqi 'BUILTIN\\Administrators.*F|NT AUTHORITY\\SYSTEM.*F' <<<"$check_acl" || die 'check.json の管理者または SYSTEM の ACL が無い'
}

assert_trace_start_after_notice() {
  local trace="$1" notice start
  notice="$(grep -n '更新の確認: 有効' "$trace" | tail -n1 | cut -d: -f1)"
  start="$(grep -n 'result=ok sc.exe start ashiato-updater' "$trace" | tail -n1 | cut -d: -f1)"
  [[ -n "$start" && -n "$notice" && "$start" -gt "$notice" ]] || die '案内より後に更新係を開始していない'
}

write_test_clock() {
  local value="$1"
  [[ "$archive" == *package-windows-test* ]] || return 0
  local hooks="$UPDATER_DIR/settings/test-hooks.json"
  python - "$hooks" "$value" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
try: value = json.loads(path.read_text())
except (FileNotFoundError, json.JSONDecodeError): value = {}
value['now'] = sys.argv[2]
path.write_text(json.dumps(value, separators=(',', ':')) + '\n')
PY
}

reset_machine() {
  local service until
  for service in ashiato-updater ashiato-server ashiato-db; do
    if windows_sc query "$service" >/dev/null 2>&1; then
      windows_sc stop "$service" >/dev/null 2>&1 || true
      until=$((SECONDS + 120))
      while [[ "$(state_code "$service")" != 1 && "$SECONDS" -lt "$until" ]]; do sleep 1; done
      windows_sc delete "$service" >/dev/null 2>&1 || true
    fi
  done
  taskkill.exe /F /IM ashiato-updater.exe >/dev/null 2>&1 || true
  taskkill.exe /F /IM ashiato-server.exe >/dev/null 2>&1 || true
  taskkill.exe /F /IM postgres.exe >/dev/null 2>&1 || true
  reg.exe delete 'HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato' /f >/dev/null 2>&1 || true
  rm -rf "$PROGRAM" "$UPDATER_DIR"
  if [[ -e 'C:/ProgramData/ashiato' ]]; then
    windows_icacls 'C:\ProgramData\ashiato' /setowner '*S-1-5-32-544' /T /C /L >/dev/null 2>&1 || true
    windows_icacls 'C:\ProgramData\ashiato' /grant '*S-1-5-32-544:(OI)(CI)F' /T /C /L >/dev/null 2>&1 || true
    rm -rf 'C:/ProgramData/ashiato'
  fi
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
  reset_machine

  start_receiver "$WORK/requests"
  echo '=== 更新係（配る zip | 試験の zip）'
  install_zip "$archive" "$out" || { cat "$out" >&2; die '更新係を含むインストールが失敗'; }

  echo '== Scenario: 更新係はサービス専用のアカウントで自動で起きる'
  assert_service ashiato-updater
  [[ "$(state_code ashiato-updater)" == 4 ]] || die '更新係が実行中でない'

  echo '== Scenario: 更新係のアカウントは状態の置き場を読めない'
  ! windows_icacls 'C:\ProgramData\ashiato' /T | grep -Fqi 'NT SERVICE\ashiato-updater' || die '状態の ACL に更新係がある'

  echo '== Scenario: 更新係の置き場は、サーバが読めて書けない'
  local updater_acl="$(windows_icacls "$(win "$UPDATER_DIR")")"
  assert_updater_acl_text "$updater_acl"

  echo '== Scenario: 切ると更新係のサービスが止まり、問い合わせが無くなる'
  local before=0; [[ -f "$WORK/requests" ]] && before="$(<"$WORK/requests")"
  write_test_clock '2027-01-01T00:00:00Z'
  "$PROGRAM/ashiato-setup.exe" update-check off >/dev/null 2>&1 || die 'update-check off が失敗'
  wait_state ashiato-updater 1
  if [[ "$archive" == *package-windows-test* ]]; then
    "$UPDATER" once >/dev/null 2>&1 || die '無効化後の 1 回目の確かめが失敗'
    write_test_clock '2027-01-02T00:00:00Z'
    "$UPDATER" once >/dev/null 2>&1 || die '無効化後の 2 回目の確かめが失敗'
  fi
  sleep 3
  local after=0; [[ -f "$WORK/requests" ]] && after="$(<"$WORK/requests")"
  [[ "$after" == "$before" ]] || die '無効化後に問い合わせが増えた'

  echo '== Scenario: 入れ直すと更新係が起きて確かめる'
  "$PROGRAM/ashiato-setup.exe" update-check on >/dev/null 2>&1 || die 'update-check on が失敗'
  wait_state ashiato-updater 4
  if [[ "$archive" == *package-windows-test* ]]; then
    write_test_clock '2027-01-03T00:00:00Z'
    wait_updater_request "$WORK/requests" "$((before + 1))"
    [[ "$(<"$WORK/requests")" == "$((before + 1))" ]] || die '有効化後の問い合わせが 1 件でない'
  fi

  echo '== Scenario: インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる'
  grep -Fq '更新の確認: 有効' "$out" || die '有効の案内が無い'
  grep -Fq 'ashiato-setup.exe update-check off' "$out" || die '切り方の案内が無い'
  [[ "$archive" == *package-windows-test* ]] && assert_trace_start_after_notice "$out"

  echo '== Scenario: 配布元に届かなくてもインストールは終わる'
  [[ "$(state_code ashiato-db)" == 4 && "$(state_code ashiato-server)" == 4 ]] || die '本体のサービスが動いていない'
  [[ -f "$UPDATER_DIR/checks.jsonl" ]] || die '更新係の記録が無い'
  if [[ "$archive" != *package-windows-test* ]]; then
    grep -Eq 'unreachable|timeout' "$UPDATER_DIR/checks.jsonl" || die '配布元に届かない記録が無い'
  fi

  echo '== Scenario: 更新係の無いインストールを走らせ直すと、更新係が足される'
  local settings_sum db_sum
  settings_sum="$(sha256sum 'C:/ProgramData/ashiato/settings/server.json' | cut -d' ' -f1)"
  db_sum="$(sha256sum 'C:/ProgramData/ashiato/db/17/PG_VERSION' | cut -d' ' -f1)"
  windows_sc stop ashiato-updater >/dev/null || true
  deadline=$((SECONDS + 120)); while [[ "$(state_code ashiato-updater)" != 1 && "$SECONDS" -lt "$deadline" ]]; do sleep 1; done
  windows_sc delete ashiato-updater >/dev/null || true
  rm -rf "$UPDATER_DIR"
  "$PROGRAM/ashiato-setup.exe" install >/dev/null 2>&1 || die '走らせ直しが失敗'
  assert_service ashiato-updater
  [[ -d "$UPDATER_DIR" ]] || die '更新係の置き場ができない'
  [[ "$(sha256sum 'C:/ProgramData/ashiato/settings/server.json' | cut -d' ' -f1)" == "$settings_sum" ]] || die '走らせ直しで設定が変わった'
  [[ "$(sha256sum 'C:/ProgramData/ashiato/db/17/PG_VERSION' | cut -d' ' -f1)" == "$db_sum" ]] || die '走らせ直しで DB が変わった'

  echo '== Scenario: 切った更新の確認は、走らせ直しでも切ったまま'
  "$PROGRAM/ashiato-setup.exe" update-check off >/dev/null 2>&1 || die '再実行前の無効化が失敗'
  "$PROGRAM/ashiato-setup.exe" install >/dev/null 2>&1 || die '無効状態の走らせ直しが失敗'
  assert_disabled_setting "$UPDATER_DIR/settings/check.json"
  assert_disabled_service ashiato-updater

  echo '== Scenario: repair は更新係に触れない'
  local updater_sum="$(sha256sum "$UPDATER_DIR/checks.jsonl" | cut -d' ' -f1)" repair_out="$WORK/repair.out"
  cp "$UPDATER_DIR/settings/check.json" "$WORK/check.json.away"
  rm "$UPDATER_DIR/settings/check.json"
  ASHIATO_SETUP_TEST_TRACE=1 "$PROGRAM/ashiato-setup.exe" repair >"$repair_out" 2>&1 || die 'repair が失敗'
  ! grep -Fq 'ashiato-updater' "$repair_out" || die 'repair の命令に更新係が出た'
  mv "$WORK/check.json.away" "$UPDATER_DIR/settings/check.json"
  assert_disabled_setting "$UPDATER_DIR/settings/check.json"
  assert_disabled_service ashiato-updater
  [[ "$(sha256sum "$UPDATER_DIR/checks.jsonl" | cut -d' ' -f1)" == "$updater_sum" ]] || die 'repair が更新係の記録を変えた'
  echo 'OK: 更新係の Windows CI の実地の確かめ'
}

self_test() {
  local work fake log body name count fake_output acl_good bad_acl
  expect_fail() { if ( "$@" ) >/dev/null 2>&1; then die "失敗すべき判定が通った: $*"; fi; }
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-self-test.XXXXXX")"
  trap 'rm -rf "${work:-}"' EXIT
  fake="$work/windows/System32"; mkdir -p "$fake" "$work/bin"
  cat >"$work/bin/cygpath" <<'EOF'
#!/usr/bin/env bash
[[ "$1" == -u ]] && printf '%s\n' "$2"
EOF
  chmod +x "$work/bin/cygpath"
  export PATH="$work/bin:$PATH"
  cat >"$fake/netsh.exe" <<'EOF'
#!/usr/bin/env bash
printf '<%s>\n' "$@"
EOF
  chmod +x "$fake/netsh.exe"
  cat >"$fake/sc.exe" <<'EOF'
#!/usr/bin/env bash
start=2; [[ "${SC_MODE:-}" == disabled ]] && start=4
account="NT SERVICE\\$2"; [[ "${SC_MODE:-}" == bad-account ]] && account='LocalSystem'
printf 'SERVICE_NAME: %s\n        START_TYPE         : %s   START\n        SERVICE_START_NAME : %s\n' "$2" "$start" "$account"
EOF
  chmod +x "$fake/sc.exe"
  cat >"$fake/icacls.exe" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *'check.json'* ]]; then
  printf 'C:\\ProgramData\\ashiato-updater\\settings\\check.json NT SERVICE\\ashiato-updater:(R)\nC:\\ProgramData\\ashiato-updater\\settings\\check.json NT SERVICE\\ashiato-server:(R)\nC:\\ProgramData\\ashiato-updater\\settings\\check.json BUILTIN\\Administrators:(F)\nC:\\ProgramData\\ashiato-updater\\settings\\check.json NT AUTHORITY\\SYSTEM:(F)\n'
elif [[ "${ACL_MODE:-}" == inherited ]]; then
  printf 'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(I)(OI)(CI)(M)\n'
elif [[ "${ACL_MODE:-}" == no-updater ]]; then
  printf 'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(RX)\n'
elif [[ "${ACL_MODE:-}" == server-write ]]; then
  printf 'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(OI)(CI)(M)\nC:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(M)\n'
else
  printf 'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(OI)(CI)(M)\nC:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(RX)\n'
fi
EOF
  chmod +x "$fake/icacls.exe"
  export WINDIR="$work/windows"
  log="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" block_outbound)"
  grep -Fq '<dir=out>' <<<"$log" && grep -Fq '<action=block>' <<<"$log" || die 'netsh の遮断引数が違う'
  grep -Fq '<remoteip=0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff>' <<<"$log" || die 'loopback 以外の範囲で遮断していない'
  fake_output="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" windows_sc qc ashiato-updater)"
  grep -Fq 'START_TYPE         : 2' <<<"$fake_output" || die '偽の sc.exe を読めない'
  fake_output="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" windows_icacls 'C:\\ProgramData\\ashiato-updater' /T)"
  grep -Fqi 'ashiato-updater' <<<"$fake_output" || die '偽の icacls を読めない'
  export SC_MODE=''
  assert_service ashiato-updater
  expect_fail assert_disabled_service ashiato-updater
  export SC_MODE=disabled
  expect_fail assert_service ashiato-updater
  export SC_MODE=bad-account
  expect_fail assert_service ashiato-updater
  export SC_MODE=disabled
  assert_disabled_service ashiato-updater
  export ACL_MODE=''
  acl_good="$(cygpath() { [[ $1 == -u ]] && printf '%s\n' "$2"; }; WINDIR="$work/windows" windows_icacls 'C:\\ProgramData\\ashiato-updater' /T)"
  assert_updater_acl_text "$acl_good"
  bad_acl=$'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(I)(OI)(CI)(M)'
  expect_fail assert_updater_acl_text "$bad_acl"
  bad_acl=$'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(RX)'
  expect_fail assert_updater_acl_text "$bad_acl"
  bad_acl=$'C:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-updater:(OI)(CI)(M)\nC:\\ProgramData\\ashiato-updater NT SERVICE\\ashiato-server:(OI)(CI)(M)'
  expect_fail assert_updater_acl_text "$bad_acl"
  printf '{"enabled": false, "since": "2027-01-01T00:00:00Z"}\n' >"$work/check.json"
  assert_disabled_setting "$work/check.json"
  printf '{"enabled": true}\n' >"$work/check.json"
  expect_fail assert_disabled_setting "$work/check.json"
  printf '更新の確認: 有効\nresult=ok sc.exe start ashiato-updater\n' >"$work/trace.good"
  assert_trace_start_after_notice "$work/trace.good"
  printf 'result=ok sc.exe start ashiato-updater\n更新の確認: 有効\n' >"$work/trace.bad"
  expect_fail assert_trace_start_after_notice "$work/trace.bad"
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
