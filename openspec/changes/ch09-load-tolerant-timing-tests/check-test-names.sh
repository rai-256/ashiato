#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
echo "Scenario: 既存の試験の名前と印は消えず、試験の本数は減らない"
m=$(git merge-base origin/main HEAD)

files=(crates collector-android/app/src web/src/__tests__ web/e2e tools/*.sh tools/smoke.d)
collect() {
  local rev=$1 pattern=$2 path=$3
  git grep -h -A8 -E "$pattern" "$rev" -- "$path" 2>/dev/null | awk '
    /Scenario:/ { print; next }
    /fn [A-Za-z0-9_]+/ { match($0, /fn [A-Za-z0-9_]+/); print substr($0,RSTART,RLENGTH); next }
    /fun (`[^`]+`|[A-Za-z0-9_]+)/ { match($0, /fun (`[^`]+`|[A-Za-z0-9_]+)/); print substr($0,RSTART,RLENGTH); next }
    /[[:space:]](it|test)\(/ { print }
  ' | sort -u
}

compare_lost() {
  local old new
  old=$(collect "$m" "$1" "$2")
  new=$(collect HEAD "$1" "$2")
  comm -23 <(printf '%s\n' "$old") <(printf '%s\n' "$new")
}
lost=$(compare_lost 'Scenario:|#\[(tokio::)?test|@Test|(^|[[:space:]])(it|test)\(' 'crates')
lost+=$'\n'$(compare_lost 'Scenario:|#\[(tokio::)?test|@Test|(^|[[:space:]])(it|test)\(' 'collector-android/app/src')
lost+=$'\n'$(compare_lost 'Scenario:|(^|[[:space:]])(it|test)\(' 'web')
lost+=$'\n'$(compare_lost 'Scenario:' 'tools')
if [[ -n "${lost//$'\n'/}" ]]; then
  echo '消えた試験名・Scenario 印:' >&2
  printf '%s\n' "$lost" | sed '/^$/d' >&2
  exit 1
fi

count_matches() { git grep -h -E "$1" "$2" -- "$3" 2>/dev/null | wc -l; }
compare_count() {
  local label=$1 pattern=$2 path=$3 b h
  b=$(count_matches "$pattern" "$m" "$path"); h=$(count_matches "$pattern" HEAD "$path")
  echo "$label $b -> $h"
  [[ "$h" -ge "$b" ]] || { echo "$label の本数が減った" >&2; return 1; }
}
compare_count rust '^[[:space:]]*#\[(tokio::)?test' crates
compare_count kotlin '^[[:space:]]*@Test' collector-android/app/src
compare_count web-unit '(^|[[:space:]])(it|test)\(' web/src/__tests__
compare_count web-e2e '(^|[[:space:]])test\(' web/e2e
compare_count windows '^[[:space:]]*#\[test\]' crates/collector-windows/tests/runtime_windows.rs
compare_count scenario 'Scenario:' tools
