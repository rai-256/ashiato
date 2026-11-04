#!/usr/bin/env bash
# CH09 の直す前の木で、実時間の試験が負荷に耐えない回を記録する。
set -u
set -o pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
OUT="$ROOT/openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt"
work=$(mktemp -d "${TMPDIR:-/tmp}/ch09-load-proof.XXXXXX")
trap 'rm -rf "$work"' EXIT

[ "${1:-}" = before ] || { echo "使い方: load-proof.sh before" >&2; exit 2; }
: >"$OUT"

probe() {
  local label=$1 package=$2 test_name=$3 load_mode=$4 log rc result
  log="$work/$label-$load_mode.log"
  set +e
  "$ROOT/tools/load-repeat.sh" --expect-fail --times 10 --load "$load_mode" -- \
    cargo test -p "$package" --lib "$test_name" >"$log" 2>&1
  rc=$?
  set -e
  if [ "$rc" -eq 0 ]; then result=yes; else result=no; fi
  {
    echo "--- $label load=$load_mode ---"
    echo "load-repeat-rc: $rc"
    sed -n '/^run [0-9].* rc=/p; /^pass /p' "$log"
  } >>"$OUT"
  printf '%s\n' "$result"
}

record() {
  local label=$1 package=$2 test_name=$3 first second loads result
  first=$(probe "$label" "$package" "$test_name" host)
  loads=none
  result=$first
  if [ "$first" = yes ]; then
    loads=host
  else
    second=$(probe "$label" "$package" "$test_name" db)
    if [ "$second" = yes ]; then
      result=yes
      loads=db
    fi
  fi
  echo "$label: $result" >>"$OUT"
  echo "load-$label: $loads" >>"$OUT"
}

record history ashiato-collector-windows history_large_fetch_keeps_observing_foreground
record scan ashiato-server archive_scan_interval_change_takes_effect_without_restart
