#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
mode=${1:-}
[[ "$mode" == before || "$mode" == after ]] && [[ $# -eq 1 ]] || {
  echo "使い方: $0 before|after" >&2
  exit 2
}

before="$ROOT/openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt"
if [[ "$mode" == before ]]; then
  set -u
  out="$before"
  work=$(mktemp -d "${TMPDIR:-/tmp}/ch09-load-proof.XXXXXX")
else
  echo "Scenario: 全コアに負荷をかけた状態で、見回りと走査の間隔の 2 本の試験が続けて 10 回通る"
  out="$ROOT/openspec/changes/ch09-load-tolerant-timing-tests/load-after.txt"
  work=$(mktemp -d "${TMPDIR:-/tmp}/ch09-load-proof-after.XXXXXX")
fi
trap 'rm -rf -- "$work"' EXIT
: > "$out"

probe() {
  local label=$1 package=$2 test_name=$3 load_mode=$4 log rc result
  log="$work/$label-$load_mode.log"
  set +e
  if [[ "$mode" == after ]]; then
    tools/load-repeat.sh --times 10 --load "$load_mode" -- cargo test -p "$package" --lib "$test_name" >"$log" 2>&1
  else
    tools/load-repeat.sh --expect-fail --times 10 --load "$load_mode" -- cargo test -p "$package" --lib "$test_name" >"$log" 2>&1
  fi
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then result=yes; else result=no; fi
  {
    echo "--- $label load=$load_mode ---"
    echo "command: cargo test -p $package --lib $test_name"
    echo "load-repeat-rc: $rc"
    sed -n '/^run [0-9].* rc=/p; /^pass /p' "$log"
    echo "test-output:"
    cat "$log"
  } >>"$out"
  if [[ "$mode" == after ]]; then
    [[ "$rc" -eq 0 ]]
  else
    printf '%s\n' "$result"
  fi
}

if [[ "$mode" == before ]]; then
  record() {
    local label=$1 package=$2 test_name=$3 first second loads result
    first=$(probe "$label" "$package" "$test_name" host)
    loads=none
    result=$first
    if [[ "$first" == yes ]]; then
      loads=host
    else
      second=$(probe "$label" "$package" "$test_name" db)
      if [[ "$second" == yes ]]; then
        result=yes
        loads=db
      fi
    fi
    echo "$label: $result" >>"$out"
    echo "load-$label: $loads" >>"$out"
  }
  record history ashiato-collector-windows history_large_fetch_keeps_observing_foreground
  record scan ashiato-server archive_scan_interval_change_takes_effect_without_restart
else
  load_for() {
    local label=$1 load_mode
    load_mode=$(awk -F': ' -v key="load-$label" '$1 == key { print $2; exit }' "$before")
    [[ -n "$load_mode" ]] || { echo "load-$label が load-before.txt にありません" >&2; return 1; }
    [[ "$load_mode" == none ]] && load_mode=host
    printf '%s\n' "$load_mode"
  }
  history_mode=$(load_for history)
  scan_mode=$(load_for scan)
  probe history ashiato-collector-windows history_large_fetch_keeps_observing_foreground "$history_mode"
  probe scan ashiato-server archive_scan_interval_change_takes_effect_without_restart "$scan_mode"
fi
cat "$out"
