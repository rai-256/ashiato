#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
[[ "${1:-}" == after && $# -eq 1 ]] || { echo "使い方: $0 after" >&2; exit 2; }

echo "Scenario: 全コアに負荷をかけた状態で、見回りと走査の間隔の 2 本の試験が続けて 10 回通る"
before="$ROOT/openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt"
out="$ROOT/openspec/changes/ch09-load-tolerant-timing-tests/load-after.txt"
work=$(mktemp -d "${TMPDIR:-/tmp}/ch09-load-proof-after.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
: >"$out"

load_for() {
  local label=$1 mode
  mode=$(awk -F': ' -v key="load-$label" '$1 == key { print $2; exit }' "$before")
  [[ -n "$mode" ]] || { echo "load-$label が load-before.txt にありません" >&2; return 1; }
  [[ "$mode" == none ]] && mode=host
  printf '%s\n' "$mode"
}

probe() {
  local label=$1 package=$2 test_name=$3 mode=$4 log rc
  log="$work/$label-$mode.log"
  set +e
  tools/load-repeat.sh --times 10 --load "$mode" -- cargo test -p "$package" --lib "$test_name" >"$log" 2>&1
  rc=$?
  set -e
  {
    echo "--- $label load=$mode ---"
    echo "command: cargo test -p $package --lib $test_name"
    echo "load-repeat-rc: $rc"
    sed -n '/^run [0-9].* rc=/p; /^pass /p' "$log"
    echo "test-output:"
    cat "$log"
  } >>"$out"
  [[ "$rc" -eq 0 ]]
}

history_mode=$(load_for history)
scan_mode=$(load_for scan)
probe history ashiato-collector-windows history_large_fetch_keeps_observing_foreground "$history_mode"
probe scan ashiato-server archive_scan_interval_change_takes_effect_without_restart "$scan_mode"
cat "$out"
