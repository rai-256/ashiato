#!/usr/bin/env bash
# 使い方: tools/load-repeat.sh [--times N] [--load host|db|none] [--expect-fail] -- <命令…>
#         tools/load-repeat.sh --self-test
# rc: 通常は全回成功なら 0、1 回でも失敗なら 1。--expect-fail はその逆。
# CH09 で、混み具合の下でも実時間を使う試験が続けて通るかを再現可能にする。
set -u

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
times=10
load=host
expect_fail=0

usage() {
  echo "使い方: tools/load-repeat.sh [--times N] [--load host|db|none] [--expect-fail] -- <命令…> | --self-test" >&2
  exit 2
}

host_load_pids=()
db_load_started=0
command_pid=''
self_test_tmp=''

stop_db_load() {
  [ "$db_load_started" -eq 1 ] || return 0
  "$ROOT/tools/db.sh" exec -T db sh -c '
    pids=$(ps -eo pid=,args= 2>/dev/null | awk '\''$2 ~ /^ch09-load-/ { print $1 }'\'' || true)
    [ -z "$pids" ] || kill $pids 2>/dev/null || true
  ' >/dev/null 2>&1 || true
  db_load_started=0
}

stop_load() {
  local pid
  for pid in "${host_load_pids[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  host_load_pids=()
  stop_db_load
  if [ -n "$self_test_tmp" ]; then
    rm -rf "$self_test_tmp"
    self_test_tmp=''
  fi
}

on_signal() {
  [ -z "$command_pid" ] || kill -TERM "$command_pid" 2>/dev/null || true
  stop_load
  exit 130
}

trap stop_load EXIT
trap on_signal INT TERM

start_load() {
  local i count
  case "$load" in
    none) ;;
    host)
      count=$(nproc)
      for ((i = 1; i <= count; i++)); do
        bash -c 'while :; do :; done' &
        host_load_pids+=("$!")
      done
      ;;
    db)
      if ! "$ROOT/tools/db.sh" exec -T db true >/dev/null 2>&1; then
        echo "DB が要る: db コンテナに接続できない" >&2
        return 1
      fi
      count=$(nproc)
      (
        trap 'trap - INT TERM; kill 0' INT TERM
        for ((i = 1; i <= count; i++)); do
          "$ROOT/tools/db.sh" exec -T -d db sh -c "exec -a ch09-load-${BASHPID}-${i} sh -c 'while :; do :; done'" >/dev/null &
        done
        wait
      ) &
      command_pid=$!
      wait "$command_pid" || return $?
      command_pid=''
      db_load_started=1
      ;;
    *)
      echo "未知の負荷: $load" >&2
      return 2
      ;;
  esac
}

run_repeat() {
  local passed=0 rc i start elapsed
  start_load || return $?
  for ((i = 1; i <= times; i++)); do
    start=$(date +%s%N)
    command_pid=''
    "$@" &
    command_pid=$!
    wait "$command_pid"
    rc=$?
    command_pid=''
    elapsed=$((($(date +%s%N) - start) / 1000000))
    echo "run $i/$times rc=$rc elapsed=${elapsed}ms"
    if [ "$expect_fail" -eq 1 ]; then
      [ "$rc" -ne 0 ] && passed=$((passed + 1))
    else
      [ "$rc" -eq 0 ] && passed=$((passed + 1))
    fi
  done
  echo "pass $passed/$times"
  if [ "$expect_fail" -eq 1 ]; then
    [ "$passed" -gt 0 ]
  else
    [ "$passed" -eq "$times" ]
  fi
}

run_expect() {
  local want=$1; shift
  local out rc
  set +e
  out=$("$@" 2>&1)
  rc=$?
  printf '%s\n' "$out" | grep -qF "$want" || { printf '%s\n' "$out" >&2; return 1; }
  printf '%s\n' "$out"
  return "$rc"
}

no_host_load_children() {
  local parent=$1 attempt pid='' args=''
  for attempt in 1 2 3 4 5; do
    for pid in $(pgrep -P "$parent" -x bash 2>/dev/null || true); do
      args=$(ps -p "$pid" -o args= 2>/dev/null || true)
      [[ "$args" == *'while :; do :; done'* ]] && break 2
    done
    if [ "$pid" = '' ] || [[ "$args" != *'while :; do :; done'* ]]; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

self_test() {
  local tmp runner rc mode db_count output
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/ch09-load-self-test.XXXXXX")
  self_test_tmp=$tmp

  echo "Scenario: 負荷の台本は、全部通れば通り、1 回でも落ちれば何回目かを挙げて落ちる"
  for mode in none host; do
    set +e
    output=$(run_expect 'pass 5/5' "$ROOT/tools/load-repeat.sh" --times 5 --load "$mode" -- true 2>&1)
    rc=$?
    [ "$rc" -eq 0 ] || { echo "self-test NG: 常成功の命令 load=$mode rc=$rc" >&2; printf '%s\n' "$output" >&2; return 1; }
    printf '0\n' >"$tmp/$mode"
    set +e
    output=$(run_expect 'pass 4/5' "$ROOT/tools/load-repeat.sh" --times 5 --load "$mode" -- sh -c 'n=$(cat "$1"); n=$((n + 1)); echo "$n" >"$1"; test "$n" -ne 3' sh "$tmp/$mode" 2>&1)
    rc=$?
    [ "$rc" -eq 1 ] || { echo "self-test NG: 3 回目だけ失敗 load=$mode rc=$rc" >&2; printf '%s\n' "$output" >&2; return 1; }
    printf '%s\n' "$output" | grep -q 'run 3/5 rc=1' || { echo "self-test NG: 3 回目の rc が出ない load=$mode" >&2; return 1; }
    printf '0\n' >"$tmp/$mode-fail"
    set +e
    output=$(run_expect 'pass 4/5' "$ROOT/tools/load-repeat.sh" --times 5 --load "$mode" -- sh -c 'n=$(cat "$1"); n=$((n + 1)); echo "$n" >"$1"; test "$n" -ne 3' sh "$tmp/$mode-fail" 2>&1)
    rc=$?
    [ "$rc" -eq 1 ] || { echo "self-test NG: 失敗命令の rc=$rc load=$mode" >&2; printf '%s\n' "$output" >&2; return 1; }
    printf '%s\n' "$output" | grep -q 'run 3/5 rc=1' || { echo "self-test NG: 失敗回の rc が出ない load=$mode" >&2; return 1; }
    if ! no_host_load_children "$$"; then
      echo "self-test NG: 負荷の子が残った: load=$mode" >&2
      return 1
    fi
  done

  echo "Scenario: 落ちることを期待する指定では、落ちる回が無ければ台本が落ちる"
  set +e
  run_expect 'pass 0/3' "$ROOT/tools/load-repeat.sh" --times 3 --load none --expect-fail -- true
  rc=$?
  [ "$rc" -eq 1 ] || return 1
  printf '0\n' >"$tmp/expected"
  set +e
  output=$(run_expect 'pass 1/3' "$ROOT/tools/load-repeat.sh" --times 3 --load none --expect-fail -- sh -c 'n=$(cat "$1"); n=$((n + 1)); echo "$n" >"$1"; test "$n" -ne 2' sh "$tmp/expected" 2>&1)
  rc=$?
  [ "$rc" -eq 0 ] || { printf '%s\n' "$output" >&2; return 1; }

  echo "Scenario: 負荷の台本を途中で止めても、かけた負荷は残らない"
  if ! "$ROOT/tools/db.sh" exec -T db true >/dev/null 2>&1; then
    echo "DB が要る: self-test の db 負荷を確認できない" >&2
    return 1
  fi
  for mode in db host; do
    timeout --signal=INT --kill-after=3 5 "$ROOT/tools/load-repeat.sh" --times 1 --load "$mode" -- sleep 60 &
    runner=$!
    set +e
    wait "$runner"
    rc=$?
    [ "$rc" -ne 0 ] || return 1
    if ! no_host_load_children "$runner"; then
      echo "負荷の子が残った: load=$mode" >&2
      return 1
    fi
    if [ "$mode" = db ]; then
      db_count=$("$ROOT/tools/db.sh" exec -T db sh -c "ps -eo args= 2>/dev/null | awk '\$1 ~ /^ch09-load-/ { n++ } END { print n + 0 }'")
      [ "$db_count" -eq 0 ] || { echo "DB の負荷が残った: $db_count" >&2; return 1; }
    fi
  done
}

if [ "${1:-}" = --self-test ]; then
  self_test
  exit $?
fi

while [ "$#" -gt 0 ]; do
  case "$1" in
    --times)
      [ "$#" -ge 2 ] || usage
      times=$2
      [[ "$times" =~ ^[1-9][0-9]*$ ]] || usage
      shift 2
      ;;
    --load)
      [ "$#" -ge 2 ] || usage
      load=$2
      shift 2
      ;;
    --expect-fail) expect_fail=1; shift ;;
    --) shift; break ;;
    *) usage ;;
  esac
done
[ "$#" -gt 0 ] || usage
run_repeat "$@"
