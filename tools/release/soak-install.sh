#!/usr/bin/env bash
# 配布後のサービス、ログ、更新確認を判定する置く段。
set -u

root="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=check-ng-observations.sh
source "$root/tools/release/check-ng-observations.sh"

# Git Bash が Windows のスイッチを POSIX の場所へ書き換えないようにする。
# `/FI` や `/grant` が化けると、置く段の観測と掃除が黙って無効になる。
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'

SOAK_SAME_ERROR_LIMIT=3
SOAK_WAIT_SECONDS=300
SOAK_COLLECTOR_RESTART_SECONDS=60
SOAK_READ_INTERVAL_SECONDS=5
NG_OBSERVED_DIR="${NG_OBSERVED_DIR:-${TMPDIR:-/tmp}}"
SOAK_SELF_TEST_WORK=''
SOAK_CLEANUP_WORK=''
SOAK_RULE='ashiato-update-check-ci-outbound'
SOAK_INSTALL_PASSWORD='soak-install-password-0123'
SOAK_PROGRAM_DIR='C:/Program Files/ashiato'
SOAK_STATE_DIR='C:/ProgramData/ashiato'
SOAK_UPDATER_DIR='C:/ProgramData/ashiato-updater'
SOAK_COLLECTOR_DIR=''
SOAK_PROFILE="${SOAK_PROFILE:-}"
SOAK_DOWNLOADS="${SOAK_DOWNLOADS:-}"

ng_secret_values() {
  local server="${SOAK_STATE_DIR:-}/settings/server.json" owner="${SOAK_STATE_DIR:-}/settings/owner.json"
  printf '%s\n' "$SOAK_INSTALL_PASSWORD"
  ng_settings_secrets "$server" "$owner"
}

soak_out() { printf '%s\n' "$*" | ng_redact; }

soak_usage() {
  cat <<'EOF'
usage: soak-install.sh <server.zip> <collector.zip>
       soak-install.sh --self-test
       soak-install.sh --list
EOF
}

soak_cleanup() {
  taskkill.exe /F /IM ashiato-collector-windows.exe >/dev/null 2>&1 || true
  rm -rf "${SOAK_CLEANUP_WORK:-}" "${SOAK_TMP:-}" "${SOAK_OBSERVED:-}"
}

soak_service_failures() {
  local dir="$1" start="$2" end="$3" events="$4" svc line ts state pid prev readable=0 total=0
  local -a services=(ashiato-db ashiato-server ashiato-updater)
  if [[ -r "$events" ]]; then
    readable=1
    while IFS='|' read -r ts line event_code; do
      [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
      [[ "$event_code" == 7031 || "$event_code" == 7034 ]] || continue
      ((total++))
    done <"$events"
  fi
  for svc in "${services[@]}"; do
    local stopped=0 changed=0 event_count=0
    prev=''
    if [[ -f "$dir/services/$svc" ]]; then
      while IFS='|' read -r ts state pid; do
        [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
        if [[ "$state" != RUNNING ]]; then ((stopped++)); fi
        if [[ -n "$prev" && "$pid" != "$prev" ]]; then ((changed++)); fi
        prev="$pid"
      done <"$dir/services/$svc"
    fi
    if [[ -r "$events" ]]; then
      while IFS='|' read -r ts line event_code; do
        [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
        [[ "$event_code" == 7031 || "$event_code" == 7034 ]] || continue
        [[ "$line" == "$svc" ]] && ((event_count++))
      done <"$events"
    fi
    soak_out "service=$svc stopped=$stopped pid_changed=$changed system_events=$event_count"
    (( stopped + changed + event_count == 0 )) || die_seen "$svc の起こし直し・停止を観測: stopped=$stopped pid_changed=$changed system_events=$event_count" "$(tail -n 80 "$dir/services/$svc" 2>/dev/null || true)"
  done
  if (( readable )); then
    soak_out "system-events: readable count=$total"
  else
    printf '%s\n' "::warning::システムの記録を読めないため、状態と PID の読みだけで判定します" >&2
    soak_out "system-events: unreadable"
  fi
}

soak_log_timestamp() {
  local ts="$1"
  date -u -d "$ts" +%s%N 2>/dev/null || true
}

soak_log_errors() {
  local start="$1" end="$2" baseline_dir="$3" log name line ts ts_ns start_ns end_ns key count line_no base_lines panic
  start_ns="$(soak_log_timestamp "$start")"; end_ns="$(soak_log_timestamp "$end")"
  for log in "$4" "$5" "$6" "$7"; do
    [[ -f "$log" ]] || continue
    name="$(basename "$log")"
    base_lines=0
    if [[ -r "$baseline_dir/$name.lines" ]]; then
      base_lines="$(cat "$baseline_dir/$name.lines")"
    fi
    line_no=0
    while IFS= read -r line; do
      ((line_no++))
      panic=0
      # まず段で絞る。Windows では各行の sed/date 起動が特に重い。
      if [[ "$name" == *server* ]]; then
        [[ "$line" == *ERROR* || "$line" == *kind=\"panic\"* || "$line" == *panic* ]] || continue
        [[ "$line" == *kind=\"panic\"* || "$line" == *panic* ]] && panic=1
      elif [[ "$name" == *db* ]]; then
        [[ "$line" =~ (ERROR:|FATAL:|PANIC:) ]] || continue
      elif [[ "$name" == *collector* ]]; then
        [[ "$line" == *ERROR* ]] || continue
      fi
      ts="$(sed -nE 's/^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+(Z|[+-][0-9]{2}:[0-9]{2})).*/\1/p; s/^([0-9]{4}-[0-9]{2}-[0-9]{2}) ([0-9:.]+) UTC.*/\1T\2Z/p' <<<"$line")"
      if [[ -n "$ts" ]]; then
        ts_ns="$(soak_log_timestamp "$ts")"
        [[ -n "$ts_ns" && "$ts_ns" -gt "$start_ns" && "$ts_ns" -lt "$end_ns" ]] || continue
      else
        (( line_no > base_lines )) || continue
      fi
      (( panic == 0 )) || die_seen "サーバのログの panic" "$line"
      key="$(printf '%s' "$line" | sed -E 's/^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+(Z|[+-][0-9]{2}:[0-9]{2})|[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9:.]+ UTC)[[:space:]]*//; s/^\[[0-9]+\][[:space:]]*//')"
      printf '%s\t%s\n' "$name" "$key" >>"$SOAK_TMP/errors"
    done <"$log"
  done
  [[ -f "$SOAK_TMP/errors" ]] || return 0
  while IFS=$'\t' read -r name key; do
    count="$(awk -F'\t' -v want_name="$name" -v want_key="$key" '$1==want_name && substr($0, length($1) + 2) == want_key {count++} END{print count+0}' "$SOAK_TMP/errors")"
    soak_out "error[$name] $key (count=$count)"
    (( count < SOAK_SAME_ERROR_LIMIT )) || die_seen "$name の同じ誤りが ${count} 回" "$(tail -n 80 "$SOAK_TMP/errors" 2>/dev/null || true)"
  done < <(awk -F'\t' '!seen[$0]++' "$SOAK_TMP/errors")
}

soak_check_records() {
  local file="$1" since="$2" label="$3" line at ok reason any=0 bad=0 only_limit=1
  [[ -f "$file" ]] || { die "${label} の確かめた記録が無い"; return; }
  while IFS= read -r line; do
    at="$(sed -nE 's/.*"at":"([^"]+)".*/\1/p' <<<"$line")"
    [[ "$at" > "$since" ]] || continue
    any=1
    ok="$(sed -nE 's/.*"ok":(true|false).*/\1/p' <<<"$line")"
    reason="$(sed -nE 's/.*"reason":"([^"]+)".*/\1/p' <<<"$line")"
    soak_out "check[$label] at=$at ok=$ok reason=${reason:-none}"
    [[ "$ok" == true ]] && continue
    bad=1
    [[ "$reason" == rate_limited ]] || only_limit=0
  done <"$file"
  if (( !any )); then
    die "${label} の置く段以後の確かめた記録が無い"
    return
  fi
  (( bad == 0 )) && return 0
  if (( only_limit )); then
    printf '%s\n' "::warning::${label} は問い合わせの上限だけでした" >&2
  else
    die "${label} の確かめが成功せず、問い合わせの上限以外の理由があります"
  fi
}

soak_collector_pid() {
  local file="$1" start="$2" restart="$3" end="$4" ts pid previous='' changes=0 allow_restart=1
  while IFS='|' read -r ts pid; do
    [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
    [[ -z "$pid" && -z "$previous" ]] && continue
    if [[ -z "$pid" ]]; then
      if [[ "$ts" > "$restart" && $allow_restart -eq 1 ]]; then allow_restart=0; else ((changes++)); fi
    elif [[ -n "$previous" && "$pid" != "$previous" ]]; then
      if [[ "$ts" > "$restart" && $allow_restart -eq 1 ]]; then allow_restart=0; else ((changes++)); fi
    fi
    [[ -n "$pid" ]] && previous="$pid"
  done <"$file"
  soak_out "collector pid changes=$changes"
  (( changes == 0 )) || die_seen "台本が止めたほかに PC の収集アプリの PID が消えた・変わった: $changes" "$(tail -n 80 "$file" 2>/dev/null || true)"
}

soak_evaluate() {
  local dir="$1" start="$2" soak_start="$3" restart="$4" end="$5" baseline_dir="${6:-$1/.soak-log-baselines}" observed previous_observed
  SOAK_TMP="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak-eval.XXXXXX")" || die '判定の一時置き場を作れない'
  observed="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak-observed.XXXXXX")" || die '観測の一時置き場を作れない'
  SOAK_OBSERVED="$observed"
  previous_observed="${NG_OBSERVED_DIR:-}"
  NG_OBSERVED_DIR="$observed"
  trap "rm -rf \"\${SOAK_TMP:-}\" \"\${SOAK_OBSERVED:-}\"; SOAK_TMP=''; SOAK_OBSERVED=''; NG_OBSERVED_DIR=\"$previous_observed\"; trap - RETURN" RETURN
  ng_secret_file "$NG_OBSERVED_DIR"
  ng_collect_secrets || die '伏せる値を読めないため、置く段の観測を続けられない'
  soak_service_failures "$dir" "$soak_start" "$end" "$dir/events"
  soak_log_errors "$soak_start" "$end" "$baseline_dir" "$dir/logs/updater.log" "$dir/logs/server.log" "$dir/logs/db.log" "$dir/logs/collector.log"
  soak_check_records "$dir/checks.jsonl" "$start" updater
  soak_check_records "$dir/update-checks.jsonl" "$restart" collector
  soak_collector_pid "$dir/collector.pid" "$start" "$restart" "$end"
  [[ "$(<"$dir/health")" == 200 ]] || die "置き終わりの healthz が 200 ではない"
  for svc in ashiato-db ashiato-server ashiato-updater; do
    tail -n 1 "$dir/services/$svc" | grep -q '|RUNNING|' || die "$svc が置き終わりに実行中でない"
  done
  [[ -f "$dir/collector.pid" ]] || die 'PC の収集アプリの PID の読みが無い'
  soak_out "collector outage_notice_unavailable=$(grep -c 'outage_notice_unavailable' "$dir/logs/collector.log" 2>/dev/null || true)"
}

# 自己試験の失敗は部分殻で受けるため、RETURN trap だけには頼らない。
soak_evaluate_isolated() {
  local rc=0
  soak_evaluate "$@" || rc=$?
  rm -rf "${SOAK_TMP:-}" "${SOAK_OBSERVED:-}"
  return "$rc"
}

soak_windows_path() { printf '%s' "${1//\//\\}"; }

soak_native_path() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi
}

soak_service_state() {
  sc.exe queryex "$1" 2>/dev/null | tr -d '\r' | awk -F ':' '
    toupper($1) ~ /STATE/ { split($2,a," "); state=a[1] }
    toupper($1) ~ /PID/ { gsub(/ /,"",$2); pid=$2 }
    END { printf "%s|%s\n", state, pid }'
}

soak_reset_machine() {
  local service state until reset_seconds="${reset_wait_seconds:-120}"
  for service in ashiato-updater ashiato-server ashiato-db ashiato-updater-dev ashiato-server-dev ashiato-db-dev; do
    if sc.exe query "$service" >/dev/null 2>&1; then
      sc.exe stop "$service" >/dev/null 2>&1 || true
      until=$((SECONDS + reset_seconds))
      while :; do
        state="$(soak_service_state "$service")"
        [[ "$state" == 1\|* || "$SECONDS" -ge "$until" ]] && break
        sleep 1
      done
      sc.exe delete "$service" >/dev/null 2>&1 || true
    fi
  done
  for process in ashiato-updater.exe ashiato-server.exe postgres.exe ashiato-collector-windows.exe; do
    taskkill.exe /F /IM "$process" >/dev/null 2>&1 || true
  done
  reg.exe delete 'HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato' /f >/dev/null 2>&1 || true
  rm -rf "$SOAK_PROGRAM_DIR" 'C:/Program Files/ashiato-dev' "$SOAK_UPDATER_DIR" 'C:/ProgramData/ashiato-updater-dev'
  for dir in "$SOAK_STATE_DIR" 'C:/ProgramData/ashiato-dev'; do
    [[ -e "$dir" ]] || continue
    icacls.exe "$(soak_windows_path "$dir")" /setowner '*S-1-5-32-544' /T /C /L >/dev/null 2>&1 || true
    icacls.exe "$(soak_windows_path "$dir")" /grant '*S-1-5-32-544:(OI)(CI)F' /T /C /L >/dev/null 2>&1 || true
    rm -rf "$dir"
  done
  [[ -n "$SOAK_COLLECTOR_DIR" ]] && rm -rf "$SOAK_COLLECTOR_DIR"
  [[ ! -e "$SOAK_PROGRAM_DIR/ashiato-server.exe" && ! -e "$SOAK_STATE_DIR/settings" && ! -e "$SOAK_UPDATER_DIR/status.json" ]] ||
    die "前の段の入れたものを消せない: $SOAK_PROGRAM_DIR $SOAK_STATE_DIR $SOAK_UPDATER_DIR"
}

soak_remove_outbound_rule() {
  local shown
  netsh.exe advfirewall firewall delete rule "name=$SOAK_RULE" >/dev/null 2>&1 || true
  shown="$(netsh.exe advfirewall firewall show rule "name=$SOAK_RULE" 2>&1 || true)"
  [[ "$shown" != *"$SOAK_RULE"* ]] || die_seen "更新係の外向きの塞ぎを外せない: $SOAK_RULE" "$shown"
}

soak_install_server() {
  local archive="$1" work="$2" setup setup_native profile_args=()
  unzip -q "$archive" -d "$work/server" || die '配るサーバの zip を開けない'
  setup="$work/server/ashiato-setup.exe"
  [[ -f "$setup" ]] || die '配るサーバの zip にインストーラが無い'
  setup_native="$(soak_native_path "$setup")"
  [[ -z "$SOAK_PROFILE" ]] || profile_args+=(--profile "$SOAK_PROFILE")
  [[ -z "$SOAK_DOWNLOADS" ]] || profile_args+=(--downloads "$SOAK_DOWNLOADS")
  printf '%s\n%s\n' "$SOAK_INSTALL_PASSWORD" "$SOAK_INSTALL_PASSWORD" |
    timeout -k 10 "${SOAK_SETUP_TIMEOUT:-600}" "$setup_native" install "${profile_args[@]}" >"$work/install.out" 2>&1 ||
    die_seen '配る形のサーバのインストールに失敗' "$(tail -n 80 "$work/install.out" 2>/dev/null || true)"
}

soak_wait_healthz() {
  local seconds="$1" deadline
  deadline=$((SECONDS + seconds))
  while [[ "$(curl.exe -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18700/healthz 2>/dev/null || true)" != 200 ]]; do
    [[ "$SECONDS" -lt "$deadline" ]] || die 'サーバが healthz に答えない'
    sleep 1
  done
}

soak_record_sample() {
  local dir="$1" now service state pid
  now="$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)"
  for service in ashiato-db ashiato-server ashiato-updater; do
    IFS='|' read -r state pid <<<"$(soak_service_state "$service")"
    [[ "$state" == 4 ]] && state=RUNNING
    printf '%s|%s|%s\n' "$now" "${state:-UNKNOWN}" "${pid:-}" >>"$dir/services/$service"
  done
  pid="$(tasklist.exe /FI 'IMAGENAME eq ashiato-collector-windows.exe' /FO CSV /NH 2>/dev/null | tr -d '\r' | sed -n 's/^"[^"]*","\([0-9][0-9]*\)".*/\1/p' | head -n 1)"
  printf '%s|%s\n' "$now" "$pid" >>"$dir/collector.pid"
}

soak_record_log_baselines() {
  local dir="$1" name path
  mkdir -p "$dir/.soak-log-baselines"
  for name in updater server db collector; do
    path="$SOAK_UPDATER_DIR/logs/updater.log"
    [[ "$name" == server ]] && path="$SOAK_STATE_DIR/logs/server"
    [[ "$name" == db ]] && path="$SOAK_STATE_DIR/logs/db"
    [[ "$name" == collector ]] && path="$SOAK_COLLECTOR_DIR/collector.log"
    if [[ -f "$path" ]]; then
      wc -l <"$path" >"$dir/.soak-log-baselines/$name.log.lines"
    elif [[ -d "$path" ]]; then
      cat "$path"/* 2>/dev/null | wc -l >"$dir/.soak-log-baselines/$name.log.lines"
    else
      printf '0\n' >"$dir/.soak-log-baselines/$name.log.lines"
    fi
  done
}

soak_collect_observations() {
  local dir="$1" name path
  mkdir -p "$dir/logs" "$dir/.soak-log-baselines"
  cp "$SOAK_UPDATER_DIR/checks.jsonl" "$dir/checks.jsonl" 2>/dev/null || :
  cp "$SOAK_COLLECTOR_DIR/update-checks.jsonl" "$dir/update-checks.jsonl" 2>/dev/null || :
  cp "$SOAK_COLLECTOR_DIR/collector.log" "$dir/logs/collector.log" 2>/dev/null || :
  for name in updater server db collector; do
    path="$SOAK_UPDATER_DIR/logs/updater.log"
    [[ "$name" == server ]] && path="$SOAK_STATE_DIR/logs/server"
    [[ "$name" == db ]] && path="$SOAK_STATE_DIR/logs/db"
    [[ "$name" == collector ]] && path="$SOAK_COLLECTOR_DIR/collector.log"
    if [[ -f "$path" ]]; then cp "$path" "$dir/logs/$name.log"; else cat "$path"/* >"$dir/logs/$name.log" 2>/dev/null || :; fi
  done
  if wevtutil.exe qe System /f:text >"$dir/events.raw" 2>/dev/null; then
    awk '
      function flush() {
        if (event_date != "" && event_code != "" && event_service != "")
          print event_date "\t" event_service "\t" event_code
        event_date = event_code = event_service = ""
      }
      /^[[:space:]]*Date[[:space:]]*:/ {
        line = $0; sub(/^[^:]*:[[:space:]]*/, "", line); event_date = line
      }
      /^[[:space:]]*Event ID[[:space:]]*:/ {
        line = $0; sub(/^[^:]*:[[:space:]]*/, "", line)
        match(line, /[0-9]+/); event_code = substr(line, RSTART, RLENGTH)
      }
      /ashiato-(db|server|updater)(-dev)?/ {
        match($0, /ashiato-(db|server|updater)(-dev)?/)
        event_service = substr($0, RSTART, RLENGTH)
      }
      /^[[:space:]]*$/ { flush() }
      END { flush() }
    ' <"$dir/events.raw" |
    while IFS=$'\t' read -r event_date event_service event_code; do
      normalized="$(date -u -d "$event_date" +%Y-%m-%dT%H:%M:%S.000Z 2>/dev/null || true)"
      [[ -n "$normalized" ]] && printf '%s|%s|%s\n' "$normalized" "$event_service" "$event_code"
    done >"$dir/events"
  else
    rm -f "$dir/events"
  fi
  rm -f "$dir/events.raw"
  printf '%s\n' "$(curl.exe -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18700/healthz 2>/dev/null || true)" >"$dir/health"
}

soak_run() {
  local server_zip="$1" collector_zip="$2" work="$3" start soak_start restart='' end collector collector_native
  mkdir -p "$work/services" "$work/observed"
  # 失敗が評価より前でも、秘密を含む観測は実地の作業場所だけに置き、cleanup で消す。
  NG_OBSERVED_DIR="$work/observed"
  if [[ -z "$SOAK_COLLECTOR_DIR" ]]; then
    if [[ -n "${LOCALAPPDATA:-}" && $(command -v cygpath 2>/dev/null || true) ]]; then
      SOAK_COLLECTOR_DIR="$(cygpath -m "$LOCALAPPDATA/ashiato/collector")"
    else
      SOAK_COLLECTOR_DIR="${LOCALAPPDATA:-C:/Users/runneradmin/AppData/Local}/ashiato/collector"
    fi
  fi
  if [[ -z "$SOAK_PROFILE" && -n "${USERPROFILE:-}" && $(command -v cygpath 2>/dev/null || true) ]]; then
    SOAK_PROFILE="$(cygpath -m "$USERPROFILE")"
  fi
  if [[ -n "$SOAK_PROFILE" && -z "$SOAK_DOWNLOADS" ]]; then
    local downloads
    downloads="$(reg.exe query 'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders' /v '{374DE290-123F-4565-9164-39C4925E467B}' 2>/dev/null | tr -d '\r' | sed -n 's/.*REG_[A-Z_]*[[:space:]]\{1,\}//p' | head -n 1)"
    downloads="${downloads//%USERPROFILE%/$USERPROFILE}"
    [[ -n "$downloads" ]] || downloads="$USERPROFILE\\Downloads"
    SOAK_DOWNLOADS="$(soak_native_path "$downloads")"
  fi
  [[ -z "$SOAK_DOWNLOADS" ]] || mkdir -p "$SOAK_DOWNLOADS"
  NG_STATE_DIR="$SOAK_STATE_DIR"
  NG_HEALTHZ_URL='http://127.0.0.1:18700/healthz'
  NG_SETTINGS_FILES="$SOAK_STATE_DIR/settings/server.json
$SOAK_STATE_DIR/settings/owner.json"
  soak_reset_machine
  start="$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)"
  soak_remove_outbound_rule
  soak_install_server "$server_zip" "$work"
  soak_wait_healthz 150
  soak_start="$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)"
  soak_record_log_baselines "$work"
  unzip -q "$collector_zip" -d "$work/collector" || die '配る PC の収集アプリの zip を開けない'
  collector="$work/collector/ashiato-collector-windows.exe"
  [[ -f "$collector" ]] || die '配る PC の収集アプリの zip に実行ファイルが無い'
  collector_native="$(soak_native_path "$collector")"
  "$collector_native" >/dev/null 2>&1 &
  soak_record_sample "$work"
  local elapsed=0
  while (( elapsed < SOAK_WAIT_SECONDS )); do
    sleep "$SOAK_READ_INTERVAL_SECONDS"; elapsed=$((elapsed + SOAK_READ_INTERVAL_SECONDS)); soak_record_sample "$work"
    if (( elapsed >= SOAK_COLLECTOR_RESTART_SECONDS )) && [[ -z "$restart" ]]; then
      taskkill.exe /F /IM ashiato-collector-windows.exe >/dev/null 2>&1 || true
      "$collector_native" >/dev/null 2>&1 &
      restart="$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)"
    fi
  done
  [[ -n "$restart" ]] || restart="$soak_start"
  end="$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)"
  soak_collect_observations "$work"
  taskkill.exe /F /IM ashiato-collector-windows.exe >/dev/null 2>&1 || true
  echo '== Scenario: 配る形のサーバを外につながる状態で 5 分置くと、落ちず・止まらず・誤りが続かず、本物の配布元に確かめる'
  echo '== Scenario: 配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける'
  soak_evaluate "$work" "$start" "$soak_start" "$restart" "$end"
}

soak_install_self_test() {
  local work="$1" bin="$1/bin" calls="$1/calls" server collector
  mkdir -p "$bin" "$work/C:/ProgramData/ashiato-updater" "$work/C:/ProgramData/ashiato/settings"
  export ST22_CALLS="$calls"; : >"$calls"
  cat >"$bin/sc.exe" <<'EOF'
#!/usr/bin/env bash
printf 'sc %s\n' "$*" >>"$ST22_CALLS"
printf 'STATE : 4 RUNNING\nPID : 42\n'
EOF
  cat >"$bin/taskkill.exe" <<'EOF'
#!/usr/bin/env bash
printf 'taskkill %s\n' "$*" >>"$ST22_CALLS"
EOF
  cat >"$bin/reg.exe" <<'EOF'
#!/usr/bin/env bash
printf 'reg %s\n' "$*" >>"$ST22_CALLS"
EOF
  cat >"$bin/icacls.exe" <<'EOF'
#!/usr/bin/env bash
printf 'icacls %s\n' "$*" >>"$ST22_CALLS"
EOF
  cat >"$bin/netsh.exe" <<'EOF'
#!/usr/bin/env bash
printf 'netsh %s\n' "$*" >>"$ST22_CALLS"
if [[ "$*" == *'show rule'* && -e "$ST22_RULE" ]]; then printf 'Rule Name: ashiato-update-check-ci-outbound\n'; fi
if [[ "$*" == *'delete rule'* && "${ST22_KEEP_RULE:-0}" != 1 ]]; then rm -f "$ST22_RULE"; fi
EOF
  cat >"$bin/wevtutil.exe" <<'EOF'
#!/usr/bin/env bash
cat <<'EVENT'
Log Name: System
Source: Service Control Manager
Date: 11/20/2026 12:00:11 AM
Event ID: 0
Task Category: None
Level: Information
Description:
The ashiato-server service is running.

EVENT
EOF
  cat >"$bin/curl.exe" <<'EOF'
#!/usr/bin/env bash
printf '200'
EOF
  cat >"$bin/tasklist.exe" <<'EOF'
#!/usr/bin/env bash
printf '"ashiato-collector-windows.exe","42","Console","1","1,000 K"\n'
EOF
  cat >"$bin/rm" <<'EOF'
#!/usr/bin/env bash
if [[ "${ST22_KEEP_STATE:-0}" == 1 && "$*" == *"$SOAK_STATE_DIR"* ]]; then exit 0; fi
exec /bin/rm "$@"
EOF
  cat >"$work/ashiato-setup.exe" <<'EOF'
#!/usr/bin/env bash
printf 'setup %s\n' "$*" >>"$ST22_CALLS"
[[ ! -e 'C:/ProgramData/ashiato-updater/status.json' ]] || exit 1
sleep 0.01
mkdir -p "$SOAK_STATE_DIR/settings" "$SOAK_STATE_DIR/logs/server" "$SOAK_STATE_DIR/logs/db"
printf '200\n' >"$SOAK_STATE_DIR/health"
mkdir -p "$SOAK_UPDATER_DIR"
printf '%s\n' '{"at":"2099-11-20T00:00:00.000Z","ok":true,"reason":null}' >"$SOAK_UPDATER_DIR/checks.jsonl"
EOF
  cat >"$work/ashiato-collector-windows.exe" <<'EOF'
#!/usr/bin/env bash
printf 'collector %s\n' "$*" >>"$ST22_CALLS"
mkdir -p "$SOAK_COLLECTOR_DIR"
printf '%s\n' '{"at":"2099-11-20T00:00:00.000Z","ok":true,"reason":null}' >"$SOAK_COLLECTOR_DIR/update-checks.jsonl"
: >"$SOAK_COLLECTOR_DIR/collector.log"
EOF
  chmod +x "$bin"/* "$work/ashiato-setup.exe" "$work/ashiato-collector-windows.exe"
  server="$work/server.zip"; collector="$work/collector.zip"
  (cd "$work" && zip -q "$server" ashiato-setup.exe && zip -q "$collector" ashiato-collector-windows.exe)
  export ST22_RULE="$work/rule"; : >"$ST22_RULE"
  printf 'leftover\n' >"$work/C:/ProgramData/ashiato-updater/status.json"
  (cd "$work" && PATH="$bin:$PATH" \
    ST22_SELF_TEST_BIN="$bin" \
    SOAK_PROGRAM_DIR='C:/Program Files/ashiato' SOAK_STATE_DIR='C:/ProgramData/ashiato' \
    SOAK_UPDATER_DIR='C:/ProgramData/ashiato-updater' SOAK_COLLECTOR_DIR='collector-state' \
    SOAK_PROFILE='C:/Users/soak' SOAK_DOWNLOADS='C:/Users/soak/Downloads' \
    reset_wait_seconds=0 SOAK_WAIT_SECONDS=1 SOAK_COLLECTOR_RESTART_SECONDS=0 SOAK_READ_INTERVAL_SECONDS=1 SOAK_SETUP_TIMEOUT=1 \
    "$root/tools/release/soak-install.sh" --self-test-run "$server" "$collector" "$work/run") || die 'self-test: 実地の口が通らない'
  grep -q '^2026-11-20T00:00:11.000Z|ashiato-server|0$' "$work/run/events" ||
    die 'self-test: wevtutil の /f:text 出力を判定用の行へ変換していない'
  grep -q '^setup install --profile .* --downloads ' "$calls" || die 'self-test: install-check と同じ profile/downloads の口で入れていない'
  local wiring reset_line rule_line setup_line
  wiring="$(declare -f soak_evaluate)"
  for name in soak_service_failures soak_log_errors soak_check_records soak_collector_pid; do
    [[ "$wiring" == *"$name"* ]] || die "self-test: 実地の判定関数を呼んでいない: $name"
  done
  reset_line="$(grep -n '^sc\|^taskkill\|^reg\|^icacls' "$calls" | head -n 1 | cut -d: -f1)"
  rule_line="$(grep -n '^netsh .*delete rule' "$calls" | head -n 1 | cut -d: -f1)"
  setup_line="$(grep -n '^setup install' "$calls" | head -n 1 | cut -d: -f1)"
  (( reset_line < rule_line && rule_line < setup_line )) || die 'self-test: 掃除・塞ぎ・インストールの順序が違う'
  [[ ! -e "$work/C:/ProgramData/ashiato-updater/status.json" ]] || die 'self-test: status.json が残っている'
  # Scenario: 置く段は、前の段の入れたものと更新係の置き場を消し、塞ぎを外してから配る zip を入れる
  # Scenario: 配る zip を入れる時に、前の段の更新係の置き場が残っていない
  grep -q 'taskkill.*ashiato-collector-windows.exe' "$calls" || die 'self-test: 収集アプリを起こし直していない'
  [[ "$(grep -c '^netsh ' "$calls")" == 2 ]] || die 'self-test: 塞ぎの削除と確認をしていない'
  grep -q '^setup install' "$calls" || die 'self-test: インストーラを呼んでいない'
  : >"$ST22_RULE"
  if (PATH="$bin:$PATH" ST22_KEEP_RULE=1 soak_remove_outbound_rule) >"$work/rule.out" 2>&1; then die 'self-test: 塞ぎが残るのに通った'; fi
  grep -q 'NG:' "$work/rule.out" || die 'self-test: 塞ぎが残った理由が出ない'
  # Scenario: 塞ぎを外せなければ、配る zip を入れずに落ちる
  : >"$ST22_RULE"; : >"$calls"
  if (cd "$work" && PATH="$bin:$PATH" ST22_SELF_TEST_BIN="$bin" SOAK_PROGRAM_DIR='C:/Program Files/ashiato' SOAK_STATE_DIR='C:/ProgramData/ashiato' SOAK_UPDATER_DIR='C:/ProgramData/ashiato-updater' SOAK_COLLECTOR_DIR='collector-state' ST22_KEEP_RULE=1 reset_wait_seconds=0 SOAK_WAIT_SECONDS=1 SOAK_COLLECTOR_RESTART_SECONDS=0 SOAK_READ_INTERVAL_SECONDS=1 SOAK_SETUP_TIMEOUT=1 "$root/tools/release/soak-install.sh" --self-test-run "$server" "$collector" "$work/rule-fail") >"$work/order.out" 2>&1; then
    die 'self-test: 塞ぎが残るのに実地の口が通った'
  fi
  ! grep -q '^setup ' "$calls" || die 'self-test: 塞ぎが残るのにインストーラを呼んだ'
  # Scenario: 消した後に前の段の入れたものが残れば、配る zip を入れずに落ちる
  mkdir -p "$work/C:/ProgramData/ashiato/settings"
  : >"$calls"
  if (cd "$work" && PATH="$bin:$PATH" ST22_SELF_TEST_BIN="$bin" ST22_KEEP_STATE=1 SOAK_WAIT_SECONDS=1 SOAK_COLLECTOR_RESTART_SECONDS=0 SOAK_READ_INTERVAL_SECONDS=1 SOAK_SETUP_TIMEOUT=1 "$root/tools/release/soak-install.sh" --self-test-run "$server" "$collector" "$work/state-fail") >"$work/state.out" 2>"$work/state.err"; then
    die 'self-test: 残った置き場を通した'
  fi
  grep -q 'C:/ProgramData/ashiato' "$work/state.err" || die 'self-test: 残った置き場の名前が標準エラーに出ない'
  ! grep -q '^netsh ' "$calls" || die 'self-test: 残った置き場なのに塞ぎの命令を呼んだ'
  ! grep -q '^setup ' "$calls" || die 'self-test: 残った置き場なのにインストーラを呼んだ'
  [[ "$(soak_list | wc -l)" -eq 2 ]] || die 'self-test: --list が実地の 2 Scenario でない'
  : >"$calls"
  if (cd "$work" && ST22_SELF_TEST_BIN="$bin" "$root/tools/release/soak-install.sh" --self-test-run "$server" "$collector" "$work/unsafe-path") >"$work/unsafe-path.out" 2>&1; then
    die 'self-test: 偽の命令が PATH の先頭に無い --self-test-run を通した'
  fi
  [[ ! -s "$calls" ]] || die 'self-test: 偽の命令が PATH の先頭に無いのに reset の命令を呼んだ'
  if (cd "$work" && PATH="$bin:$PATH" env -u ST22_CALLS "$root/tools/release/soak-install.sh" --self-test-run "$server" "$collector" "$work/unsafe") >"$work/unsafe.out" 2>&1; then
    die 'self-test: 偽の命令の印無しで --self-test-run を通した'
  fi
}

soak_ci_definition_self_test() {
  # Scenario: 置く段は、インストールと更新の試験の手順より後に走る
  # Scenario: 更新係の外向きを塞ぐ手順は、最初のインストールの試験の手順より前のままである
  # Scenario: 置く段の手順と、Windows のインストールの試験の job に時間の上限がある
  # Scenario: インストールと更新の試験と置く段の 4 つの台本の Scenario の突き合わせは残る
  # Scenario: 網に出る試験を走らせる手順は、握手の試験の 1 つだけである
  # Scenario: 更新係の外向きの塞ぎを外す手順は、置く段の 1 つだけである
  local root_dir="$1"
  python3 - "$root_dir" <<'PY'
from pathlib import Path
import re
import sys
root = Path(sys.argv[1])
files = sorted((root / '.github' / 'workflows').glob('*.yml')) + sorted((root / '.github' / 'workflows').glob('*.yaml'))
text = '\n'.join(p.read_text(encoding='utf-8') for p in files)
ci = (root / '.github' / 'workflows' / 'ci.yml').read_text(encoding='utf-8')
def run_blocks(source):
    lines = source.splitlines()
    blocks = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.lstrip()
        if (stripped.startswith('run:') or stripped.startswith('- run:')) and not stripped.startswith('#'):
            indent = len(line) - len(stripped)
            block = []
            inline = stripped.split('run:', 1)[1].strip()
            if inline and inline not in ('|', '>') and not inline.startswith('#'):
                block.append(inline)
            i += 1
            while i < len(lines):
                child = lines[i]
                if child.strip() and len(child) - len(child.lstrip()) <= indent:
                    break
                if not child.lstrip().startswith('#'):
                    block.append(child)
                i += 1
            blocks.append('\n'.join(block))
            continue
        i += 1
    return blocks

def check(value):

    blocks = run_blocks(value)
    commands = '\n'.join(blocks)
    assert 'soak-install.sh --self-test' in commands
    ignored_count = sum(block.count('--ignored') + block.count('--include-ignored') for block in blocks)
    assert ignored_count == 1
    assert sum(('--ignored' in block or '--include-ignored' in block) and 'tls_' in block for block in blocks) == 1
    assert sum(len(re.findall(r'(?m)^.*soak-install\.sh\s+(?!-)', block)) for block in blocks) == 1
    assert sum('--block-outbound' in block for block in blocks) == 1
    assert sum('ashiato-update-check-ci-outbound' in block and ('delete rule' in block or 'Remove-NetFirewallRule' in block) for block in blocks) == 0
    for script in ('install-check.sh', 'update-check-install.sh', 'update-apply-install.sh', 'soak-install.sh'):
        assert f'{script} --list' in commands
    assert 'update-check-install.sh --block-outbound' in commands
    assert 'install-check.sh --list' in commands
    assert 'soak-install.sh package/' in commands
    assert 'update-apply-install.sh \\' in commands
    assert commands.index('update-check-install.sh --block-outbound') < commands.index('install-check.sh --list')
    assert commands.index('soak-install.sh package/') > commands.index('update-apply-install.sh \\')
    assert '- name: 配る形を外につながる状態で置く' in value
    soak_step_start = value.index('- name: 配る形を外につながる状態で置く')
    assert '\n      # spec の Scenario' in value[soak_step_start:]
    soak_step_end = value.index('\n      # spec の Scenario', soak_step_start)
    assert 'timeout-minutes: 15' in value[soak_step_start:soak_step_end]
    assert 'install-windows:\n    timeout-minutes: 100' in value
assert files
check(ci)
all_blocks = []
for workflow in files:
    all_blocks.extend(run_blocks(workflow.read_text(encoding='utf-8')))
all_commands = '\n'.join(all_blocks)
assert sum(block.count('--ignored') + block.count('--include-ignored') for block in all_blocks) == 1
assert sum(('--ignored' in block or '--include-ignored' in block) and 'tls_' in block for block in all_blocks) == 1
assert sum(len(re.findall(r'(?m)^.*soak-install\.sh\s+(?!-)', block)) for block in all_blocks) == 1
assert sum('--block-outbound' in block for block in all_blocks) == 1
assert sum('ashiato-update-check-ci-outbound' in block and ('delete rule' in block or 'Remove-NetFirewallRule' in block) for block in all_blocks) == 0
assert text.count('soak-install.sh package/') == 1
def must_fail(old, new):
    assert old in ci
    broken = ci.replace(old, new, 1)
    assert broken != ci
    try:
        check(broken)
    except AssertionError:
        return
    raise AssertionError('壊した CI の写しを自己試験が通した')
must_fail('soak-install.sh package/', 'soak-install.sh broken/')
must_fail('update-check-install.sh --block-outbound', 'update-check-install.sh --no-block-outbound')
must_fail('timeout-minutes: 15', 'timeout-minutes: 14')
must_fail('soak-install.sh --list', 'soak-install.sh --scenarios')
must_fail('timeout-minutes: 100', 'timeout-minutes: 99')
must_fail('tools/release/soak-install.sh --list', 'tools/release/soak-install.sh --list --ignored')
must_fail('soak-install.sh package/', 'netsh.exe advfirewall firewall delete rule name=ashiato-update-check-ci-outbound\n          soak-install.sh package/')
must_fail('soak-install.sh package/', 'Remove-NetFirewallRule -Name ashiato-update-check-ci-outbound\n          soak-install.sh package/')
PY
}

soak_self_test() {
  local work="$1" base
  export NG_OBSERVED_DIR="$work/observed"
  export TMPDIR="$work/tmp"
  mkdir -p "$TMPDIR"
  base="$work/base"
  mkdir -p "$base/services" "$base/logs"
  printf '2026-11-20T00:00:00.000Z|RUNNING|10\n2026-11-20T00:00:15.000Z|RUNNING|10\n2026-11-20T00:00:20.000Z|RUNNING|10\n' >"$base/services/ashiato-db"
  cp "$base/services/ashiato-db" "$base/services/ashiato-server"; cp "$base/services/ashiato-db" "$base/services/ashiato-updater"
  printf '200\n' >"$base/health"; printf '2026-11-20T00:00:00.000Z|ashiato-db|0\n' >"$base/events"
  printf '{"at":"2026-11-20T00:00:01.000Z","ok":true,"reason":null,"latest":null}\n' >"$base/checks.jsonl"
  printf '{"at":"2026-11-20T00:00:11.000Z","ok":true,"reason":null,"latest":null}\n' >"$base/update-checks.jsonl"
  printf '2026-11-20T00:00:05.000Z|99\n2026-11-20T00:00:11.000Z|100\n2026-11-20T00:01:10.000Z|100\n' >"$base/collector.pid"
  : >"$base/logs/updater.log"; : >"$base/logs/server.log"; : >"$base/logs/db.log"; : >"$base/logs/collector.log"
  mkdir -p "$base/.soak-log-baselines"
  for log in updater.log server.log db.log collector.log; do printf '0\n' >"$base/.soak-log-baselines/$log.lines"; done

  soak_install_self_test "$work" || die '置く段の実地の自己試験が通らない'
  soak_ci_definition_self_test "$root" || die 'self-test: CI の置く段の定義が通らない'
  [[ "$SOAK_WAIT_SECONDS" == 300 ]] || die 'self-test: SOAK_WAIT_SECONDS is not 300'

  # Scenario: 置く間に 1 回でも起こし直されたら、サービスの名前と回数を出して落ちる
  cp -R "$base" "$work/restart"; sed -i '3s/|10$/|11/' "$work/restart/services/ashiato-server"; expect_fail "$work/restart" "ashiato-server"
  # Scenario: 起こし直されずに止まったままのサービスがあれば落ちる
  cp -R "$base" "$work/stopped"; sed -i '2s/RUNNING/STOPPED/' "$work/stopped/services/ashiato-db"; expect_fail "$work/stopped" "ashiato-db"
  # Scenario: 置き終わりに healthz が 200 でなければ落ちる
  cp -R "$base" "$work/health"; printf '503\n' >"$work/health/health"; expect_fail "$work/health"
  # Scenario: 置く間より前のサービスの終わりの記録は数えない
  cp -R "$base" "$work/old-event"; printf '2026-11-19T23:59:00.000Z|ashiato-db|7031\n' >"$work/old-event/events"; expect_pass "$work/old-event" "system-events: readable count=0"
  # Scenario: システムの記録を読めなければ、警告を出し、状態とプロセスの番号で判定する
  cp -R "$base" "$work/no-event"; rm "$work/no-event/events"; expect_pass "$work/no-event" "system-events: unreadable"
  # The service state is healthy; a 7031/7034 record alone must still fail.
  cp -R "$base" "$work/event-only"; printf '2026-11-20T00:00:11.000Z|ashiato-server|7034\n' >"$work/event-only/events"; expect_fail "$work/event-only" "ashiato-server"
  # Scenario: 同じ誤りが 3 回あると落ちる
  [[ "$SOAK_SAME_ERROR_LIMIT" == 3 ]] || die "self-test: SOAK_SAME_ERROR_LIMIT is not 3"; cp -R "$base" "$work/three"; for i in 1 2 3; do printf '2026-11-20T00:01:0%s.000Z ERROR same\n' "$i" >>"$work/three/logs/updater.log"; done; expect_fail "$work/three"
  # Scenario: 2 回の同じ誤りは、行を出して通る
  cp -R "$base" "$work/two"; for i in 1 2; do printf '2026-11-20T00:01:0%s.000Z ERROR same\n' "$i" >>"$work/two/logs/updater.log"; done; expect_pass "$work/two"
  # Scenario: 警告の段の行と置く段の始まりより前の行は数えない
  cp -R "$base" "$work/warn"; for i in 1 2 3 4 5; do printf '2026-11-19T00:00:0%s.000Z ERROR same\n' "$i" >>"$work/warn/logs/collector.log"; done; expect_pass "$work/warn"; sed -i 's/2026-11-19T00:00/2026-11-20T00:01/' "$work/warn/logs/collector.log"; expect_fail "$work/warn"
  # Scenario: 置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない
  cp -R "$base" "$work/before-soak"; for i in 1 2 3 4 5; do printf '2026-11-20T00:00:0%s.000Z FATAL: same\n' "$i" >>"$work/before-soak/logs/db.log"; done; expect_pass "$work/before-soak"; sed -i 's/00:00:0/00:01:0/' "$work/before-soak/logs/db.log"; expect_fail "$work/before-soak"
  # Scenario: プロセスの番号だけが違う DB のログの行は、同じ誤りとして数える
  cp -R "$base" "$work/db-pid"; printf '2026-11-20 00:01:01.000 UTC [1] ERROR: same\n2026-11-20 00:01:02.000 UTC [2] ERROR: same\n2026-11-20 00:01:03.000 UTC [3] ERROR: same\n' >"$work/db-pid/logs/db.log"; expect_fail "$work/db-pid"
  # Unreadable timestamps are counted only when appended after the recorded baseline.
  cp -R "$base" "$work/unreadable-log"; printf '%s\n' 'bad-time ERROR same' 'bad-time ERROR same' 'bad-time ERROR same' >>"$work/unreadable-log/logs/updater.log"; expect_fail "$work/unreadable-log"; sed -i '$d' "$work/unreadable-log/logs/updater.log"; expect_pass "$work/unreadable-log"
  # Directory logs need their initial line count too; otherwise old unreadable lines are counted.
  mkdir -p "$work/baseline-dir/state/logs/server" "$work/baseline-dir/out"
  printf '%s\n%s\n' 'old unreadable ERROR one' 'old unreadable ERROR two' >"$work/baseline-dir/state/logs/server/server.log"
  (SOAK_STATE_DIR="$work/baseline-dir/state" SOAK_UPDATER_DIR="$work/baseline-dir/updater" SOAK_COLLECTOR_DIR="$work/baseline-dir/collector" soak_record_log_baselines "$work/baseline-dir/out")
  [[ "$(<"$work/baseline-dir/out/.soak-log-baselines/server.log.lines")" == 2 ]] || die 'self-test: サーバのログの基準行数がディレクトリを数えていない'
  # Scenario: サーバのログの panic の行は 1 行でも落ちる
  cp -R "$base" "$work/panic"; printf '2026-11-20T00:01:01.000Z kind="panic" boom\n' >"$work/panic/logs/server.log"; expect_fail "$work/panic"
  # Scenario: 置く段の始まりより後に確かめた行があれば通る
  expect_pass "$base"
  # Scenario: 置く段の始まりより後の確かめた記録が 1 行も無ければ落ちる
  cp -R "$base" "$work/no-check"; : >"$work/no-check/checks.jsonl"; expect_fail "$work/no-check"
  # Scenario: 届かない・読めない答えの行だけなら落ちる
  cp -R "$base" "$work/unreadable"; printf '{"at":"2026-11-20T00:01:00.000Z","ok":false,"reason":"unreadable","latest":null}\n' >"$work/unreadable/checks.jsonl"; expect_fail "$work/unreadable" "問い合わせの上限以外"
  # Scenario: 問い合わせの上限の行だけなら、警告を出して通る
  cp -R "$base" "$work/rate"; printf '{"at":"2026-11-20T00:01:00.000Z","ok":false,"reason":"rate_limited","latest":null}\n' >"$work/rate/checks.jsonl"; expect_pass "$work/rate" "::warning::updater は問い合わせの上限だけでした"
  # Scenario: 問い合わせの上限と届かないの行が混ざれば落ちる
  cp -R "$base" "$work/mixed"; printf '%s\n' '{"at":"2026-11-20T00:01:00.000Z","ok":false,"reason":"rate_limited","latest":null}' '{"at":"2026-11-20T00:01:01.000Z","ok":false,"reason":"unreachable","latest":null}' >"$work/mixed/checks.jsonl"; expect_fail "$work/mixed" "問い合わせの上限以外"
  # Scenario: 台本が止めたほかに PC の収集アプリが終われば落ちる
  cp -R "$base" "$work/collector-pid"; printf '2026-11-20T00:01:20.000Z|102\n' >>"$work/collector-pid/collector.pid"; expect_fail "$work/collector-pid"
  printf '2026-11-20T00:01:20.000Z|\n' >>"$work/collector-pid/collector.pid"; expect_fail "$work/collector-pid"
  # Scenario: 起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす
  cp -R "$base" "$work/collector-before"; printf '{"at":"2026-11-20T00:00:05.000Z","ok":false,"reason":"unreachable","latest":null}\n' >"$work/collector-before/update-checks.jsonl"; expect_fail "$work/collector-before" "collector"
  # Scenario: 置く段の出力に秘密の値が出ない
  mkdir -p "$work/secrets/settings" "$work/secrets/observed" "$work/secrets/logs"; printf '{"API_TOKEN":"secret-api","WEB_PASSWORD_VERIFIER":"secret-web","DATABASE_URL":"postgres://u:secret-db@x/db"}\n' >"$work/secrets/settings/server.json"; printf '{"DATABASE_OWNER_URL":"postgres://o:secret-owner@x/db"}\n' >"$work/secrets/settings/owner.json"; cp -R "$base" "$work/secrets/state"; printf '2026-11-20T00:01:01.000Z ERROR secret-api secret-web secret-db secret-owner password=soak-install-password-0123\n' >"$work/secrets/logs/server.log"; printf '2026-11-20 00:01:01.000 UTC [1] ERROR: secret-api secret-web secret-db secret-owner password=soak-install-password-0123\n' >"$work/secrets/logs/db.log"; export SOAK_STATE_DIR="$work/secrets" NG_OBSERVED_DIR="$work/secrets/observed"; expect_fail "$work/secrets"; ! grep -R -F -e secret-api -e secret-web -e secret-db -e secret-owner -e soak-install-password-0123 "$work/secrets/out" "$work/secrets/err" || die "self-test: secret leaked"; unset SOAK_STATE_DIR NG_OBSERVED_DIR
  grep -q '\*\*\*' "$work/secrets/out" "$work/secrets/err" || die "self-test: redaction marker missing"
  # Scenario: 置く段の台本は観測を出さずに落ちる道を持たない
  "$root/tools/release/check-ng-observations.sh" "$0" >/dev/null || die "self-test: observation checker failed"
  ! find "$TMPDIR" -mindepth 1 \( -name 'ch22-soak-eval.*' -o -name 'ch22-soak-observed.*' \) | grep -q . || die 'self-test: 判定の一時置き場が残った'
}

expect_fail() { local d="$1" needle="${2:-}"; if (trap 'rm -rf "${SOAK_TMP:-}" "${SOAK_OBSERVED:-}"' EXIT; soak_evaluate_isolated "$d" 2026-11-20T00:00:00.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:02:00.000Z) >"$d/out" 2>"$d/err"; then die "self-test: fail case unexpectedly passed: $d"; fi; grep -q 'NG:' "$d/err" || die "self-test: fail case had no NG: $d"; [[ -z "$needle" || "$(cat "$d/err")" == *"$needle"* ]] || die "self-test: missing failure detail: $d"; }
expect_pass() { local d="$1" needle="${2:-}"; (trap 'rm -rf "${SOAK_TMP:-}" "${SOAK_OBSERVED:-}"' EXIT; soak_evaluate_isolated "$d" 2026-11-20T00:00:00.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:02:00.000Z) >"$d/out" 2>"$d/err" || die "self-test: pass case failed: $d"; [[ -z "$needle" || "$(cat "$d/out" "$d/err")" == *"$needle"* ]] || die "self-test: missing output detail: $d"; }

soak_list() {
  cat <<'EOF'
配る形のサーバを外につながる状態で 5 分置くと、落ちず・止まらず・誤りが続かず、本物の配布元に確かめる
配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける
EOF
}

soak_self_test_command_guard() {
  local cmd expected actual
  [[ -n "${ST22_CALLS:-}" && -f "$ST22_CALLS" && -n "${ST22_SELF_TEST_BIN:-}" && -d "$ST22_SELF_TEST_BIN" ]] ||
    die '--self-test-run は自己試験の偽の命令の印と置き場が必要です'
  for cmd in sc.exe taskkill.exe reg.exe icacls.exe netsh.exe wevtutil.exe curl.exe tasklist.exe rm; do
    expected="$ST22_SELF_TEST_BIN/$cmd"
    actual="$(type -P "$cmd" 2>/dev/null || true)"
    [[ -x "$expected" && "$actual" == "$expected" ]] ||
      die "--self-test-run は偽の命令を PATH の先頭に置く必要があります: $cmd (expected=$expected actual=${actual:-none})"
  done
}

main() {
  trap soak_cleanup EXIT
  case "${1:-}" in
    --self-test-run)
      soak_self_test_command_guard
      SOAK_PROGRAM_DIR='C:/Program Files/ashiato' SOAK_STATE_DIR='C:/ProgramData/ashiato' SOAK_UPDATER_DIR='C:/ProgramData/ashiato-updater' SOAK_COLLECTOR_DIR='collector-state'
      export SOAK_STATE_DIR
      SOAK_WAIT_SECONDS=2 SOAK_COLLECTOR_RESTART_SECONDS=0 SOAK_READ_INTERVAL_SECONDS=1 SOAK_SETUP_TIMEOUT=1 reset_wait_seconds=0
      soak_run "$2" "$3" "$4"
      ;;
    --self-test) SOAK_SELF_TEST_WORK="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak.XXXXXX")" || die '自己試験の置き場を作れない'; SOAK_CLEANUP_WORK="$SOAK_SELF_TEST_WORK"; trap soak_cleanup EXIT; mkdir -p "$SOAK_SELF_TEST_WORK/observed"; soak_self_test "$SOAK_SELF_TEST_WORK"; NG_OBSERVED_DIR="$SOAK_SELF_TEST_WORK/observed" "$root/tools/release/check-ng-observations.sh" "$0" || die '観測の検査に失敗'; rm -f "$SOAK_SELF_TEST_WORK/observed/.ng-secrets"*; echo 'OK: soak-install self-test' ;;
    --list) soak_list ;;
    --evaluate) soak_evaluate "$2" "$3" "$4" "$5" "$6" ;;
    *)
      [[ "$#" -eq 2 ]] || { soak_usage; die 'サーバ zip と収集アプリ zip が必要です'; }
      [[ "${GITHUB_ACTIONS:-}" == true ]] || die '置く段は GitHub Actions の runner でのみ実行できます'
      [[ -f "$1" && -f "$2" ]] || die '配る zip を読む前に確認できない'
      local work server_zip collector_zip
      work="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak-run.XXXXXX")" || die '実地の置き場を作れない'
      SOAK_CLEANUP_WORK="$work"
      server_zip="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" || die 'サーバ zip の道を読めない'
      collector_zip="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")" || die '収集アプリ zip の道を読めない'
      soak_run "$server_zip" "$collector_zip" "$work"
      ;;
  esac
}
main "$@"
