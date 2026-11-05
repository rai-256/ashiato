#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
INVENTORY=openspec/changes/ch09-load-tolerant-timing-tests/inventory.md

die() { echo "check-inventory: $*" >&2; exit 1; }

rust_candidates() {
  rg -l 'elapsed\(\)|Instant::now\(\)|recv_timeout\(|for _ in 0\.\.[0-9]+' crates --glob '*.rs' 2>/dev/null || true
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
  awk -v path="$1" 'index($0, sprintf("%c%s", 96, path)) { found=1 } END { exit !found }' "$INVENTORY"
}

condition_files() {
  awk -F'|' '$0 ~ /^\|/ && $3 ~ /条件の待ち/ && $4 !~ /直さない|維持/ {
    s=$2
    if (match(s, /\140[^\140]+\140/)) {
      path=substr(s, RSTART + 1, RLENGTH - 2)
      sub(/:.*/, "", path)
      print path
    }
  }' "$INVENTORY" | sort -u
}

check_short_rust_waits() {
  local file=$1
  awk '
    /for _ in 0\.\.[0-9]+/ { line=NR; ok=($0 ~ /timing-ok:/ || prev ~ /timing-ok:/) }
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
    if (match($0, /timeout:[[:space:]]*[0-9_]+/)) {
      n=substr($0, RSTART)
      sub(/^[^0-9]*/, "", n)
      sub(/[^0-9_].*$/, "", n)
      gsub(/_/, "", n)
    } else n=""
    if (n != "" && (n + 0) < 30000 && prev !~ /timing-ok:/ && $0 !~ /timing-ok:/)
      print FILENAME ":" NR ": Playwrightの30秒未満の期限"
  } { prev=$0 }' "$file"
}
check_short_shell_waits() {
  local file=$1
  awk '
    function mark_ok(line) { return line ~ /timing-ok:/ }
    function loop_count(line,    n,a,b) {
      if (match(line, /seq[[:space:]]+[0-9]+[[:space:]]+([0-9]+)/)) {
        n=substr(line, RSTART); sub(/^seq[[:space:]]+[0-9]+[[:space:]]+/, "", n); sub(/[^0-9].*/, "", n); return n + 0
      }
      if (match(line, /\{[0-9]+\.\.[0-9]+\}/)) {
        n=substr(line, RSTART); sub(/^\{[0-9]+\.\./, "", n); sub(/\}.*/, "", n); return n + 0
      }
      return 0
    }
    function sleep_seconds(line,    n) {
      if (!match(line, /sleep[[:space:]]+[0-9]+([.][0-9]+)?/)) return 0
      n=substr(line, RSTART); sub(/^sleep[[:space:]]+/, "", n); sub(/[^0-9.].*/, "", n); return n + 0
    }
    /for[[:space:]]+[[:alnum:]_]+[[:space:]]+in/ {
      count=loop_count($0); loop_line=NR; loop_ok=mark_ok($0) || mark_ok(prev)
    }
    NR >= loop_line && NR <= loop_line + 5 && count > 0 {
      seconds=sleep_seconds($0)
      if (seconds > 0 && count * seconds < 30 && !loop_ok && !mark_ok($0) && !mark_ok(prev))
        print FILENAME ":" NR ": bashの回数×sleepが30秒未満"
    }
    { prev=$0 }
  ' "$file"
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
  local found
  while IFS= read -r file; do
    [ -f "$file" ] || continue
    case "$file" in
      *.rs) found=$(check_short_rust_waits "$file") ;;
      *.kt) found=$(check_short_kotlin_waits "$file") ;;
      *.ts|*.tsx) found=$(check_short_playwright_waits "$file") ;;
      *.sh) found=$(check_short_shell_waits "$file") ;;
      *) found='' ;;
    esac
    [ -n "$found" ] && violations+="$found"$'\n'
  done < <(condition_files)
  if [ -n "$violations" ]; then
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
    mkdir -p "$tmp/openspec/changes/ch09-load-tolerant-timing-tests" "$tmp/crates/sample" "$tmp/web/e2e" "$tmp/tools/smoke.d"
    script="$tmp/openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh"
    cp "${BASH_SOURCE[0]}" "$script"
    chmod +x "$script"
    printf '%s\n' 'fn sample() { for _ in 0..2 { std::thread::sleep(std::time::Duration::from_millis(1)); } }' > "$tmp/crates/sample/sample.rs"
    printf '%s\n' 'test("sample", async () => { await expect(x).toBeVisible({ timeout: 5_000 }); });' > "$tmp/web/e2e/sample.spec.ts"
    printf '%s\n' 'for _ in $(seq 1 2); do sleep 1; done' > "$tmp/tools/smoke.d/sample.sh"
    cat > "$tmp/openspec/changes/ch09-load-tolerant-timing-tests/inventory.md" <<'EOF'
# self-test
| 場所 | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/sample/sample.rs:1` | 条件の待ち | 直した | self-test |
| `web/e2e/sample.spec.ts:1` | 条件の待ち | 直した | self-test |
| `tools/smoke.d/sample.sh:1` | 条件の待ち | 直した | self-test |
EOF
    if (cd "$tmp" && "$script") >/dev/null 2>&1; then
      die '--self-test は違反を見逃した'
    fi
    sed -i '1s/^/\/\/ timing-ok: self-test\n/' "$tmp/crates/sample/sample.rs"
    sed -i '1s/^/\/\/ timing-ok: self-test\n/' "$tmp/web/e2e/sample.spec.ts"
    sed -i '1s/^/# timing-ok: self-test\n/' "$tmp/tools/smoke.d/sample.sh"
    (cd "$tmp" && "$script") >/dev/null
    echo 'check-inventory self-test: PASS'
    ;;
  '') check_inventory ;;
  *) echo "使い方: $0 [--list|--self-test]" >&2; exit 2 ;;
esac
