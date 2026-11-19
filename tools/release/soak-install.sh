#!/usr/bin/env bash
# 配布後のサービス、ログ、更新確認を判定する置く段。
set -u

root="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=check-ng-observations.sh
source "$root/tools/release/check-ng-observations.sh"

SOAK_SAME_ERROR_LIMIT=3
SOAK_WAIT_SECONDS=300
SOAK_COLLECTOR_RESTART_SECONDS=60
SOAK_READ_INTERVAL_SECONDS=5
NG_OBSERVED_DIR="${NG_OBSERVED_DIR:-${TMPDIR:-/tmp}}"
SOAK_SELF_TEST_WORK=''

ng_secret_values() {
  local server="${SOAK_STATE_DIR:-}/settings/server.json" owner="${SOAK_STATE_DIR:-}/settings/owner.json"
  ng_settings_secrets "$server" "$owner" || return 1
  printf '%s\n' "${SOAK_RELEASE_PASSWORD:-}" "${SOAK_TEST_PASSWORD:-}"
}

soak_out() { printf '%s\n' "$*" | ng_redact; }

soak_usage() {
  cat <<'EOF'
usage: soak-install.sh <server.zip> <collector.zip>
       soak-install.sh --self-test
       soak-install.sh --list
EOF
}

soak_service_failures() {
  local dir="$1" start="$2" end="$3" events="$4" svc line ts state pid prev readable=0 total=0
  local -a services=(ashiato-db ashiato-server ashiato-updater)
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
      readable=1
      while IFS='|' read -r ts line _; do
        [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
        [[ "$line" == "$svc" ]] && ((event_count++))
        ((total++))
      done <"$events"
    else
      readable=0
    fi
    soak_out "service=$svc stopped=$stopped pid_changed=$changed system_events=$event_count"
    (( stopped + changed + event_count == 0 )) || die_seen "$svc の起こし直し・停止を観測: stopped=$stopped pid_changed=$changed system_events=$event_count" "$dir/services/$svc"
  done
  if (( readable )); then
    soak_out "system-events: readable count=$total"
  else
    printf '%s\n' "::warning::システムの記録を読めないため、状態と PID の読みだけで判定します" >&2
    soak_out "system-events: unreadable"
  fi
}

soak_log_errors() {
  local start="$1" end="$2" log name line ts key count line_no base_lines
  for log in "$3" "$4" "$5" "$6"; do
    [[ -f "$log" ]] || continue
    name="$(basename "$log")"
    base_lines="$(wc -l <"$log")"
    line_no=0
    while IFS= read -r line; do
      ((line_no++))
      ts="$(sed -nE 's/^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z).*/\1/p; s/^([0-9]{4}-[0-9]{2}-[0-9]{2}) ([0-9:.]+) UTC.*/\1T\2Z/p' <<<"$line")"
      if [[ -n "$ts" ]]; then
        [[ "$ts" > "$start" && "$ts" < "$end" ]] || continue
      else
        (( line_no > base_lines )) || continue
      fi
      if [[ "$name" == *server* ]]; then
        [[ "$line" == *ERROR* || "$line" == *kind=\"panic\"* || "$line" == *panic* ]] || continue
        [[ "$line" == *kind=\"panic\"* || "$line" == *panic* ]] && die_seen "サーバのログの panic" "$line"
      elif [[ "$name" == *db* ]]; then
        [[ "$line" =~ (ERROR:|FATAL:|PANIC:) ]] || continue
      elif [[ "$name" == *updater* || "$name" == *collector* ]]; then
        [[ "$line" == *ERROR* ]] || continue
      fi
      key="$(printf '%s' "$line" | sed -E 's/^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z|[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9:.]+ UTC)[[:space:]]*//; s/^\[[0-9]+\][[:space:]]*//')"
      printf '%s|%s\n' "$name" "$key" >>"$SOAK_TMP/errors"
    done <"$log"
  done
  [[ -f "$SOAK_TMP/errors" ]] || return 0
  while IFS='|' read -r name key; do
    count="$(awk -F'|' -v want_name="$name" -v want_key="$key" '$1==want_name && $2==want_key {count++} END{print count+0}' "$SOAK_TMP/errors")"
    soak_out "error[$name] $key (count=$count)"
    (( count < SOAK_SAME_ERROR_LIMIT )) || die_seen "$name の同じ誤りが ${count} 回" "$SOAK_TMP/errors"
  done < <(awk -F'|' '!seen[$0]++' "$SOAK_TMP/errors")
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
    if [[ "$label" == collector ]]; then
      soak_out "check[$label] no post-restart record"
      return 0
    fi
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
  local file="$1" restart="$2" end="$3" ts pid previous='' changes=0
  while IFS='|' read -r ts pid; do
    [[ "$ts" > "$restart" && "$ts" < "$end" ]] || continue
    if [[ -z "$pid" ]]; then
      ((changes++))
    elif [[ -n "$previous" && "$pid" != "$previous" ]]; then
      ((changes++))
    fi
    [[ -n "$pid" ]] && previous="$pid"
  done <"$file"
  soak_out "collector pid changes=$changes"
  (( changes == 0 )) || die_seen "台本が止めたほかに PC の収集アプリの PID が消えた・変わった: $changes" "$file"
}

soak_evaluate() {
  local dir="$1" start="$2" soak_start="$3" restart="$4" end="$5"
  SOAK_TMP="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak-eval.XXXXXX")" || die '判定の一時置き場を作れない'
  trap 'rm -rf "${SOAK_TMP:-}"' RETURN EXIT
  ng_secret_file "${NG_OBSERVED_DIR:-$SOAK_TMP}"; ng_collect_secrets || true
  soak_service_failures "$dir" "$soak_start" "$end" "$dir/events"
  soak_log_errors "$soak_start" "$end" "$dir/logs/updater.log" "$dir/logs/server.log" "$dir/logs/db.log" "$dir/logs/collector.log"
  soak_check_records "$dir/checks.jsonl" "$start" updater
  soak_check_records "$dir/update-checks.jsonl" "$restart" collector
  soak_collector_pid "$dir/collector.pid" "$restart" "$end"
  [[ "$(<"$dir/health")" == 200 ]] || die "置き終わりの healthz が 200 ではない"
  for svc in ashiato-db ashiato-server ashiato-updater; do
    tail -n 1 "$dir/services/$svc" | grep -q '|RUNNING|' || die "$svc が置き終わりに実行中でない"
  done
  [[ -f "$dir/collector.pid" ]] || die 'PC の収集アプリの PID の読みが無い'
  soak_out "collector outage_notice_unavailable=$(grep -c 'outage_notice_unavailable' "$dir/logs/collector.log" 2>/dev/null || true)"
}

soak_self_test() {
  local work="$1" base
  export NG_OBSERVED_DIR="$work/observed"
  base="$work/base"
  mkdir -p "$base/services" "$base/logs"
  printf '2026-11-20T00:00:00.000Z|RUNNING|10\n2026-11-20T00:00:15.000Z|RUNNING|10\n' >"$base/services/ashiato-db"
  cp "$base/services/ashiato-db" "$base/services/ashiato-server"; cp "$base/services/ashiato-db" "$base/services/ashiato-updater"
  printf '200\n' >"$base/health"; printf '2026-11-20T00:00:00.000Z|ashiato-db|0\n' >"$base/events"
  printf '{"at":"2026-11-20T00:00:01.000Z","ok":true,"reason":null,"latest":null}\n' >"$base/checks.jsonl"
  printf '{"at":"2026-11-20T00:00:11.000Z","ok":true,"reason":null,"latest":null}\n' >"$base/update-checks.jsonl"
  printf '2026-11-20T00:00:11.000Z|100\n2026-11-20T00:01:10.000Z|100\n' >"$base/collector.pid"
  : >"$base/logs/updater.log"; : >"$base/logs/server.log"; : >"$base/logs/db.log"; : >"$base/logs/collector.log"

  # Scenario: 置く間に 1 回でも起こし直されたら、サービスの名前と回数を出して落ちる
  cp -R "$base" "$work/restart"; sed -i '2s/RUNNING/STOPPED/' "$work/restart/services/ashiato-server"; expect_fail "$work/restart" "ashiato-server"
  # Scenario: 起こし直されずに止まったままのサービスがあれば落ちる
  cp -R "$base" "$work/stopped"; sed -i '2s/RUNNING/STOPPED/' "$work/stopped/services/ashiato-db"; expect_fail "$work/stopped" "ashiato-db"
  # Scenario: 置き終わりに healthz が 200 でなければ落ちる
  cp -R "$base" "$work/health"; printf '503\n' >"$work/health/health"; expect_fail "$work/health"
  # Scenario: 置く間より前のサービスの終わりの記録は数えない
  cp -R "$base" "$work/old-event"; printf '2026-11-19T23:59:00.000Z|ashiato-db|7031\n' >"$work/old-event/events"; expect_pass "$work/old-event" "system-events: readable count=0"
  # Scenario: システムの記録を読めなければ、警告を出し、状態とプロセスの番号で判定する
  cp -R "$base" "$work/no-event"; rm "$work/no-event/events"; expect_pass "$work/no-event" "system-events: unreadable"
  # Scenario: 同じ誤りが 3 回あると落ちる
  [[ "$SOAK_SAME_ERROR_LIMIT" == 3 ]] || die "self-test: SOAK_SAME_ERROR_LIMIT is not 3"; cp -R "$base" "$work/three"; for i in 1 2 3; do printf '2026-11-20T00:01:0%s.000Z ERROR same\n' "$i" >>"$work/three/logs/updater.log"; done; expect_fail "$work/three"
  # Scenario: 2 回の同じ誤りは、行を出して通る
  cp -R "$base" "$work/two"; for i in 1 2; do printf '2026-11-20T00:01:0%s.000Z ERROR same\n' "$i" >>"$work/two/logs/updater.log"; done; expect_pass "$work/two"
  # Scenario: 警告の段の行と置く段の始まりより前の行は数えない
  cp -R "$base" "$work/warn"; printf '2026-11-19T00:00:00.000Z WARN same\n' >"$work/warn/logs/updater.log"; expect_pass "$work/warn"
  # Scenario: 置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない
  cp -R "$base" "$work/before-soak"; printf '2026-11-20T00:00:01.000Z ERROR same\n' >"$work/before-soak/logs/updater.log"; expect_pass "$work/before-soak"
  # Scenario: プロセスの番号だけが違う DB のログの行は、同じ誤りとして数える
  cp -R "$base" "$work/db-pid"; printf '2026-11-20T00:01:01.000Z [1] ERROR: same\n2026-11-20T00:01:02.000Z [2] ERROR: same\n2026-11-20T00:01:03.000Z [3] ERROR: same\n' >"$work/db-pid/logs/db.log"; expect_fail "$work/db-pid"
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
  # Scenario: 起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす
  cp -R "$base" "$work/collector-before"; printf '{"at":"2026-11-20T00:00:05.000Z","ok":false,"reason":"unreachable","latest":null}\n' >"$work/collector-before/update-checks.jsonl"; expect_pass "$work/collector-before"
  # Scenario: 置く段の出力に秘密の値が出ない
  mkdir -p "$work/secrets/settings" "$work/secrets/observed" "$work/secrets/logs"; printf '{"API_TOKEN":"secret-api","WEB_PASSWORD_VERIFIER":"secret-web","DATABASE_URL":"postgres://u:secret-db@x/db"}\n' >"$work/secrets/settings/server.json"; printf '{"DATABASE_OWNER_URL":"postgres://o:secret-owner@x/db"}\n' >"$work/secrets/settings/owner.json"; cp -R "$base" "$work/secrets/state"; printf '2026-11-20T00:01:01.000Z ERROR secret-api secret-web secret-db secret-owner\n' >"$work/secrets/logs/server.log"; export SOAK_STATE_DIR="$work/secrets" NG_OBSERVED_DIR="$work/secrets/observed"; expect_fail "$work/secrets"; ! grep -R -F -e secret-api -e secret-web -e secret-db -e secret-owner "$work/secrets/out" "$work/secrets/err" || die "self-test: secret leaked"; unset SOAK_STATE_DIR NG_OBSERVED_DIR
  grep -q '\*\*\*' "$work/secrets/out" "$work/secrets/err" || die "self-test: redaction marker missing"
  # Scenario: 置く段の台本は観測を出さずに落ちる道を持たない
  "$root/tools/release/check-ng-observations.sh" "$0" >/dev/null || die "self-test: observation checker failed"
}

expect_fail() { local d="$1" needle="${2:-}"; if (soak_evaluate "$d" 2026-11-20T00:00:00.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:02:00.000Z) >"$d/out" 2>"$d/err"; then die "self-test: fail case unexpectedly passed: $d"; fi; grep -q 'NG:' "$d/err" || die "self-test: fail case had no NG: $d"; [[ -z "$needle" || "$(cat "$d/err")" == *"$needle"* ]] || die "self-test: missing failure detail: $d"; }
expect_pass() { local d="$1" needle="${2:-}"; (soak_evaluate "$d" 2026-11-20T00:00:00.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:00:10.000Z 2026-11-20T00:02:00.000Z) >"$d/out" 2>"$d/err" || die "self-test: pass case failed: $d"; [[ -z "$needle" || "$(cat "$d/out" "$d/err")" == *"$needle"* ]] || die "self-test: missing output detail: $d"; }

soak_list() {
  cat <<'EOF'
置く間に 1 回でも起こし直されたら、サービスの名前と回数を出して落ちる
起こし直されずに止まったままのサービスがあれば落ちる
置き終わりに healthz が 200 でなければ落ちる
置く間より前のサービスの終わりの記録は数えない
システムの記録を読めなければ、警告を出し、状態とプロセスの番号で判定する
同じ誤りが 3 回あると落ちる
2 回の同じ誤りは、行を出して通る
警告の段の行と置く段の始まりより前の行は数えない
置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない
プロセスの番号だけが違う DB のログの行は、同じ誤りとして数える
サーバのログの panic の行は 1 行でも落ちる
置く段の始まりより後に確かめた行があれば通る
置く段の始まりより後の確かめた記録が 1 行も無ければ落ちる
届かない・読めない答えの行だけなら落ちる
問い合わせの上限の行だけなら、警告を出して通る
問い合わせの上限と届かないの行が混ざれば落ちる
台本が止めたほかに PC の収集アプリが終われば落ちる
起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす
置く段の出力に秘密の値が出ない
置く段の台本は観測を出さずに落ちる道を持たない
EOF
}

main() {
  case "${1:-}" in
    --self-test) SOAK_SELF_TEST_WORK="$(mktemp -d "${TMPDIR:-/tmp}/ch22-soak.XXXXXX")" || die '自己試験の置き場を作れない'; trap 'rm -rf "$SOAK_SELF_TEST_WORK"' EXIT; mkdir -p "$SOAK_SELF_TEST_WORK/observed"; soak_self_test "$SOAK_SELF_TEST_WORK"; NG_OBSERVED_DIR="$SOAK_SELF_TEST_WORK/observed" "$root/tools/release/check-ng-observations.sh" "$0" || die '観測の検査に失敗'; rm -f "$SOAK_SELF_TEST_WORK/observed/.ng-secrets"*; echo 'OK: soak-install self-test' ;;
    --list) soak_list ;;
    --evaluate) soak_evaluate "$2" "$3" "$4" "$5" "$6" ;;
    *) soak_usage; die '実地の口は Task 2 で接続されます' ;;
  esac
}
main "$@"
