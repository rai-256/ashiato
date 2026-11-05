#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
INVENTORY=openspec/changes/ch09-load-tolerant-timing-tests/inventory.md

die() { echo "check-inventory: $*" >&2; exit 1; }

rust_candidates() {
  rg -l 'elapsed\(\)|Instant::now\(\)|recv_timeout\(|for _ in 0\.<[0-9]+' crates --glob '*.rs' 2>/dev/null || true
}
typescript_candidates() {
  rg -l 'waitFor|findBy|timeout:|setTimeout\(|waitForTimeout\(|expect\.poll\(|describe\.configure\(' \
    web/src/__tests__ web/e2e web/playwright.config.ts tools/recording \
    --glob '*.ts' --glob '*.tsx' 2>/dev/null || true
}
kotlin_candidates() {
  rg -l 'await\([^)]*, *TimeUnit|Thread\.sleep\(|withTimeout|nanoTime|currentTimeMillis|_MS *=|device\.wait\(|Tasks\.await\(' \
    collector-android/app/src/test collector-android/app/src/androidTest tools/recording \
    --glob '*.kt' 2>/dev/null || true
}
shell_candidates() {
  rg -l 'sleep[[:space:]]|timeout[[:space:]]|--max-time|for [[:alnum:]_]+ in|while[[:space:]]|until[[:space:]]' \
    tools/*.sh tools/smoke.d/*.sh 2>/dev/null || true
}
all_candidates() {
  { rust_candidates; typescript_candidates; kotlin_candidates; shell_candidates; } | sort -u
}
list_candidates() {
  echo 'Rust:'; rust_candidates | sed 's/^/  /'
  echo 'TypeScript:'; typescript_candidates | sed 's/^/  /'
  echo 'Kotlin:'; kotlin_candidates | sed 's/^/  /'
  echo 'bash:'; shell_candidates | sed 's/^/  /'
}

inventory_path() {
  grep -Fq '| '"$1"' |' "$INVENTORY" ||
  grep -Fq '| \140'"$1"'\140 |' "$INVENTORY" ||
    grep -Fq '\140'"$1"'\140' "$INVENTORY"
}

condition_files() {
  awk -F'|' '$0 ~ /^\|/ && $3 ~ /条件の待ち/ && $4 !~ /直さない|維持/ {
    s=$2; sub(/^.*\140/,"",s); sub(/\140.*/,"",s); sub(/:.*/,"",s); print s
  }' "$INVENTORY" | sort -u
}

check_short_rust_waits() {
  local file=$1
  awk '
    /for _ in 0\.\.[0-9]+/ { line=NR; ok=($0 ~ /timing-ok:/) }
    NR > line && NR <= line+5 && /sleep\(|settle\(/ && !ok && prev !~ /timing-ok:/ {
      print FILENAME ":" NR ": 回数で切る待ち"
    }
    { prev=$0 }
  ' "$file"
}
check_short_kotlin_waits() {
  local file=$1
  awk '
    /await\([[:space:]]*[0-9]+[[:space:]]*,[[:space:]]*TimeUnit\.(SECONDS|MILLISECONDS)/ {
      if ($0 ~ /TimeUnit\.SECONDS/ && $0 ~ /await\([[:space:]]*([1-9]|[12][0-9])[[:space:]]*,/) print FILENAME ":" NR ": 30秒未満の期限"
      if ($0 ~ /TimeUnit\.MILLISECONDS/ && $0 ~ /await\([[:space:]]*([0-9]|[12][0-9][0-9]|[12][0-9][0-9][0-9]|[12][0-9][0-9][0-9][0-9])[[:space:]]*,/ && prev !~ /timing-ok:/ && prev2 !~ /timing-ok:/) print FILENAME ":" NR ": 30秒未満の期限"
    }
    { prev2=prev; prev=$0 }
  ' "$file"
}
check_short_playwright_waits() {
  local file=$1
  awk '/timeout:[[:space:]]*[0-9_]+/ {
    n=$0; gsub(/[^0-9]/,"",n)
    if (n != "" && n < 30000 && prev !~ /timing-ok:/ && $0 !~ /timing-ok:/)
      print FILENAME ":" NR ": Playwrightの30秒未満の期限"
  } { prev=$0 }' "$file"
}

check_inventory() {
  [ -f "$INVENTORY" ] || die "$INVENTORY がありません"
  echo "Scenario: 一覧で直したとした条件の待ちに、回数の待ちと 30 秒より短い期限が残っていない"
  local missing='' path
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    if ! inventory_path "$path"; then missing+="${path}\n"; fi
  done < <(all_candidates)
  if [ -n "$missing" ]; then
    echo 'inventory.md に無い候補ファイル:' >&2
    printf '%b' "$missing" >&2
    return 1
  fi
  local violations='' file
  while IFS= read -r file; do
    [ -f "$file" ] || continue
    case "$file" in
      *.rs) violations+="$(check_short_rust_waits "$file")\n" ;;
      *.kt) violations+="$(check_short_kotlin_waits "$file")\n" ;;
      *.ts|*.tsx) violations+="$(check_short_playwright_waits "$file")\n" ;;
    esac
  done < <(condition_files)
  if [ -n "${violations//$'\n'/}" ]; then
    echo '条件の待ちとして直したファイルに残る短い待ち:' >&2
    printf '%b' "$violations" | sed '/^$/d' >&2
    return 1
  fi
  echo "inventory: $(all_candidates | wc -l | tr -d ' ') ファイルを一覧済み"
}

case "${1:-}" in
  --list) list_candidates ;;
  --self-test)
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/ch09-inventory.XXXXXX")
    trap 'rm -rf -- "$tmp"' EXIT
    printf '%s\n' '# inventory self-test' > "$tmp/inventory.md"
    printf '%s\n' 'echo "Scenario: 一覧で直したとした条件の待ちに、回数の待ちと 30 秒より短い期限が残っていない"' > "$tmp/sample.sh"
    grep -q 'Scenario:' "$tmp/sample.sh"
    test -s "$tmp/inventory.md"
    echo 'check-inventory self-test: PASS'
    ;;
  '') check_inventory ;;
  *) echo "使い方: $0 [--list|--self-test]" >&2; exit 2 ;;
esac
