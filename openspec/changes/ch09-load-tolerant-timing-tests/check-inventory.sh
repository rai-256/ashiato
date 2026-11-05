#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"
INVENTORY=openspec/changes/ch09-load-tolerant-timing-tests/inventory.md

die() { echo "check-inventory: $*" >&2; exit 1; }

# 道具が無いと候補が 0 件に見えて通ってしまう（review/code.md R1）。先に名指して落ちる。
# 候補は grep で拾う（rg はこのマシンに入っていないことがあり、依存しない）。
for tool in grep awk sort find sed wc; do
  command -v "$tool" >/dev/null 2>&1 || die "道具が無い: $tool（PATH=$PATH）"
done

# grep -rlE の rc: 0 = 当たり、1 = 当たり無し、2 以上 = 誤り（道具や場所の誤りを 0 件として飲み込まない）
search() {
  local pattern=$1 rc=0
  shift
  grep -rlE -e "$pattern" "$@" || rc=$?
  [ "$rc" -le 1 ] || die "grep が失敗した（rc=$rc）: $pattern $*"
}

rust_candidates() {
  search 'elapsed\(\)|Instant::now\(\)|recv_timeout\(|for _ in 0\.\.[0-9]+' --include='*.rs' crates
}
typescript_candidates() {
  search 'waitFor|findBy|timeout:|setTimeout\(|waitForTimeout\(|expect\.poll\(|describe\.configure\(' \
    --include='*.ts' --include='*.tsx' \
    web/src/__tests__ web/e2e web/playwright.config.ts tools/recording
}
kotlin_candidates() {
  search 'await\([^)]*, *TimeUnit|Thread\.sleep\(|withTimeout|nanoTime|currentTimeMillis|_MS *=|device\.wait\(|Tasks\.await\(' \
    --include='*.kt' \
    collector-android/app/src/test collector-android/app/src/androidTest tools/recording
}
shell_candidates() {
  search 'sleep[[:space:]]|timeout[[:space:]]|--max-time|for [[:alnum:]_]+ in|while[[:space:]]|until[[:space:]]' \
    tools/*.sh tools/smoke.d/*.sh
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
  awk -F'|' -v path="$1" '
    $0 ~ /^\|/ {
      cell=$2
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", cell)
      gsub(/`/, "", cell)
      if (index(cell, path) == 1) found=1
    }
    END { exit !found }
  ' "$INVENTORY"
}

# 表の行ごとに「行番号<TAB>型<TAB>方針<TAB>場所の欄のパス（空白区切り）」を出す。
# パスは場所の欄の中の「/ を含む語」（バッククォートの有無に依らない）。`:行` から後ろは落とす。
inventory_rows() {
  awk -F'|' '
    $0 ~ /^\|/ {
      cell=$2
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", cell)
      if (cell ~ /^-+$/ || cell ~ /^場所/) next
      text=cell
      gsub(/`/, " ", text)
      gsub(/[・、（）()]/, " ", text)
      n=split(text, words, /[[:space:]]+/)
      paths=""
      for (i = 1; i <= n; i++) {
        w=words[i]
        if (w !~ /\//) continue
        sub(/:.*/, "", w)
        if (w !~ /^[A-Za-z0-9_.*\/-]+$/) continue
        paths=paths (paths == "" ? "" : " ") w
      }
      print NR "\t" $3 "\t" $4 "\t" paths
    }
  ' "$INVENTORY"
}

# 場所のパスをファイルへ解決する。ディレクトリは中の試験・台本のファイルに、* はその一致に広げる。
resolve_path() {
  local path=$1 match found=0
  if [[ "$path" == *'*'* ]]; then
    for match in $path; do
      [ -e "$match" ] || continue
      found=1
      resolve_path "$match"
    done
    [ "$found" -eq 1 ] || return 1
  elif [ -d "$path" ]; then
    find "$path" -type f \( -name '*.rs' -o -name '*.kt' -o -name '*.ts' -o -name '*.tsx' -o -name '*.sh' \) | sort
  elif [ -f "$path" ]; then
    printf '%s\n' "$path"
  else
    return 1
  fi
}

# 行のパスが実在するか（review/code.md R2）。解決できない行を黙って飛ばさない。
check_rows_resolve() {
  local line type policy paths path bad=''
  while IFS=$'\t' read -r line type policy paths; do
    if [ -z "$paths" ]; then
      bad+="${INVENTORY}:${line}: 場所の欄にパスが無い"$'\n'
      continue
    fi
    for path in $paths; do
      if ! resolve_path "$path" >/dev/null; then
        bad+="${INVENTORY}:${line}: 実在しないパス: ${path}"$'\n'
      fi
    done
  done < <(inventory_rows)
  if [ -n "$bad" ]; then
    echo 'inventory.md の解決できない行:' >&2
    printf '%s' "$bad" >&2
    return 1
  fi
}

condition_files() {
  local line type policy paths path
  while IFS=$'\t' read -r line type policy paths; do
    [[ "$type" == *条件の待ち* ]] || continue
    [[ "$policy" == *直さない* || "$policy" == *維持* ]] && continue
    for path in $paths; do
      resolve_path "$path" || true
    done
  done < <(inventory_rows) | sort -u
}

# Rust の試験のコード（`*_tests.rs`・`tests/` の下・`testdb.rs` は全体、ほかは `#[cfg(test)] mod tests` から後ろ）。
rust_test_files() {
  find crates -type f -name '*.rs' -not -path '*/target/*' | sort
}

# 回数で切る待ち（design D4）。`for <変数> in <a>..<b>` の本体（波括弧の対応で終わりまで）が
# 眠り（`sleep(`・`settle(`）と抜け（`break`・`return`）を持ち、実時間の期限（`Instant::now()`・`deadline`・`elapsed()`）を見ないもの。
#  - 回数 × 眠りが計算でき 30 秒以上なら許す
#  - `// timing-ok:` は、ループの変数で時刻を進める（`for sec in …` で `t(sec)`）ものにだけ効く。
#    `for _ in` は時刻を兼ねないので、印があっても条件で抜ける回数の待ちとして落とす（review/code.md R4）
check_rust_waits() {
  local file=$1 whole=0
  case "$file" in
    *_tests.rs|*/tests/*|*/testdb.rs) whole=1 ;;
  esac
  awk -v whole="$whole" '
    function num(s) { sub(/[ui](8|16|32|64|128|size)$/, "", s); gsub(/_/, "", s); return s + 0 }
    function ms_of(line,    s, total) {
      total = 0
      s = line
      while (match(s, /from_millis\([0-9_]+\)/)) { total += num(substr(s, RSTART + 12, RLENGTH - 13)); s = substr(s, RSTART + RLENGTH) }
      s = line
      while (match(s, /from_secs\([0-9_]+\)/)) { total += 1000 * num(substr(s, RSTART + 10, RLENGTH - 11)); s = substr(s, RSTART + RLENGTH) }
      s = line
      while (match(s, /settle\([0-9_]+\)/)) { total += num(substr(s, RSTART + 7, RLENGTH - 8)); s = substr(s, RSTART + RLENGTH) }
      s = line
      while (match(s, /sleep\([A-Z][A-Z0-9_]*\)/)) {
        name = substr(s, RSTART + 6, RLENGTH - 7)
        if (name in consts) total += consts[name]; else unknown = 1
        s = substr(s, RSTART + RLENGTH)
      }
      return total
    }
    function finish(    secs) {
      if (inloop && has_sleep && has_exit && !has_deadline) {
        if (count > 0 && sleep_ms > 0 && !unknown && count * sleep_ms >= 30000) { inloop = 0; return }
        if (marked && var != "_") { inloop = 0; return }
        secs = (count > 0 && sleep_ms > 0 && !unknown) ? sprintf("%.1f 秒", count * sleep_ms / 1000) : "計算できない"
        printf "%s:%d: 回数で切る待ち（for %s in …、回数 × 眠り = %s。実時間の期限 30 秒にする）%s\n", FILENAME, start, var, secs, (marked ? "。timing-ok があっても for _ の条件の待ちは残せない" : "")
      }
      inloop = 0
    }
    FNR == NR {
      if (match($0, /const[[:space:]]+[A-Z][A-Z0-9_]*[[:space:]]*:[[:space:]]*(std::time::)?Duration[[:space:]]*=[[:space:]]*(std::time::)?Duration::from_(millis|secs)\([0-9_]+\)/)) {
        d = substr($0, RSTART, RLENGTH)
        name = d; sub(/^const[[:space:]]+/, "", name); sub(/[[:space:]]*:.*/, "", name)
        v = d; sub(/.*\(/, "", v); sub(/\).*/, "", v)
        consts[name] = (d ~ /from_secs/) ? 1000 * num(v) : num(v)
      }
      next
    }
    FNR == 1 { in_test = whole; cfg = 0 }
    !in_test {
      if ($0 ~ /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/) { cfg = 1; p2 = p1; p1 = $0; next }
      if (cfg && $0 ~ /^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+tests([[:space:]]|$)/) in_test = 1
      cfg = 0
      p2 = p1; p1 = $0
      next
    }
    {
      if (!inloop && match($0, /for[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]+in[[:space:]]+[A-Za-z0-9_]+[[:space:]]*\.\.=?[[:space:]]*[A-Za-z0-9_]+/)) {
        head = substr($0, RSTART, RLENGTH)
        var = head; sub(/^for[[:space:]]+/, "", var); sub(/[[:space:]].*/, "", var)
        range = head; sub(/^.*[[:space:]]in[[:space:]]+/, "", range)
        lo = range; sub(/[[:space:]]*\.\..*/, "", lo)
        hi = range; sub(/^.*\.\.=?[[:space:]]*/, "", hi)
        count = 0
        if (lo ~ /^[0-9_]+$/ && hi ~ /^[0-9_]+$/) count = num(hi) - num(lo) + (range ~ /\.\.=/ ? 1 : 0)
        inloop = 1; start = FNR; depth = 0; opened = 0
        has_sleep = 0; has_exit = 0; has_deadline = 0; sleep_ms = 0; unknown = 0
        marked = ($0 ~ /timing-ok:/ || p1 ~ /timing-ok:/ || p2 ~ /timing-ok:/)
      }
      if (inloop) {
        line = $0
        sub(/\/\/.*/, "", line)
        if (line ~ /sleep\(|settle\(/) { has_sleep = 1; sleep_ms += ms_of(line) }
        if (line ~ /(^|[^A-Za-z_])(break|return)([^A-Za-z_]|$)/) has_exit = 1
        if (line ~ /Instant::now\(\)|deadline|\.elapsed\(\)/) has_deadline = 1
        o = gsub(/\{/, "{", line); c = gsub(/\}/, "}", line)
        depth += o - c
        if (o > 0) opened = 1
        if (opened && depth <= 0) finish()
      }
      p2 = p1; p1 = $0
    }
    END { finish() }
  ' "$file" "$file"
}

# 30 秒より短い待ちの上限（design D4・D6・D17）。同じ行か 1〜2 行上の `// timing-ok:` で残せる。
check_kotlin_waits() {
  local file=$1
  awk '
    function num(s) { gsub(/_/, "", s); sub(/[lL]$/, "", s); return s + 0 }
    function report(what) { printf "%s:%d: %s\n", FILENAME, FNR, what }
    {
      ok = ($0 ~ /timing-ok:/ || p1 ~ /timing-ok:/ || p2 ~ /timing-ok:/)
      if (!ok) {
        if (match($0, /await\([[:space:]]*[0-9_]+[lL]?[[:space:]]*,[[:space:]]*(TimeUnit\.)?SECONDS/)) {
          v = substr($0, RSTART + 6); sub(/[[:space:]]*,.*/, "", v)
          if (num(v) < 30) report("30 秒未満の期限（await " v " 秒）")
        }
        if (match($0, /await\([[:space:]]*[0-9_]+[lL]?[[:space:]]*,[[:space:]]*(TimeUnit\.)?MILLISECONDS/)) {
          v = substr($0, RSTART + 6); sub(/[[:space:]]*,.*/, "", v)
          if (num(v) < 30000) report("30 秒未満の期限（await " v " ms）")
        }
        if (match($0, /[A-Z][A-Z0-9_]*_MS[[:space:]]*(:[[:space:]]*Long)?[[:space:]]*=[[:space:]]*[0-9_]+[lL]?/)) {
          d = substr($0, RSTART, RLENGTH)
          name = d; sub(/[[:space:]]*[:=].*/, "", name)
          v = d; sub(/.*=[[:space:]]*/, "", v)
          if (name !~ /POLL|INTERVAL/ && num(v) < 30000) report("30 秒未満の _MS 定数（" name " = " v "）")
        }
        if (match($0, /device\.wait\(.*,[[:space:]]*[0-9_]+[lL]?[[:space:]]*\)/)) {
          v = substr($0, RSTART, RLENGTH); sub(/[[:space:]]*\)$/, "", v); sub(/.*,[[:space:]]*/, "", v)
          if (num(v) < 30000) report("30 秒未満の device.wait（" v " ms）")
        }
        if (match($0, /SECONDS\.toNanos\([0-9_]+[lL]?\)/)) {
          v = substr($0, RSTART + 16, RLENGTH - 17)
          if ($0 !~ /MILLISECONDS\.toNanos/ && num(v) < 30) report("30 秒未満の期限（toNanos " v " 秒）")
        }
        if (match($0, /MILLISECONDS\.toNanos\([0-9_]+[lL]?\)/)) {
          v = substr($0, RSTART + 21, RLENGTH - 22)
          if (num(v) < 30000) report("30 秒未満の期限（toNanos " v " ms）")
        }
        if (match($0, /[Tt]ook[A-Za-z]*[[:space:]]*<[[:space:]]*[0-9_]+[lL]?/)) {
          v = substr($0, RSTART, RLENGTH); sub(/.*<[[:space:]]*/, "", v)
          if (num(v) < 30000) report("30 秒未満の打ち切りの上限（< " v " ms。相手と上限を一緒に延ばす）")
        }
      }
      p2 = p1; p1 = $0
    }
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
  local candidates count
  candidates=$(all_candidates) || die '候補を拾えない'
  count=$(printf '%s' "$candidates" | grep -c . || true)
  [ "$count" -gt 0 ] || die '候補が 0 件（拾う形か場所が壊れている）'

  local missing='' path
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    if ! inventory_path "$path"; then missing+="${path}"$'\n'; fi
  done <<<"$candidates"
  if [ -n "$missing" ]; then
    echo 'inventory.md に無い候補ファイル:' >&2
    printf '%s' "$missing" >&2
    return 1
  fi

  check_rows_resolve

  local violations='' file found files
  files=$(condition_files) || die '条件の待ちの行を解決できない'
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
      *.ts|*.tsx) found=$(check_short_playwright_waits "$file") ;;
      *.sh) found=$(check_short_shell_waits "$file") ;;
      *) found='' ;;
    esac
    [ -z "$found" ] || violations+="$found"$'\n'
  done <<<"$files"
  # Rust と Kotlin は一覧の分類に依らず、試験のコードを全部見る（分類の誤りで取り残しを通さない。review/code.md R3）
  while IFS= read -r file; do
    found=$(check_rust_waits "$file")
    [ -z "$found" ] || violations+="$found"$'\n'
  done < <(rust_test_files)
  while IFS= read -r file; do
    found=$(check_kotlin_waits "$file")
    [ -z "$found" ] || violations+="$found"$'\n'
  done < <(find collector-android/app/src/test collector-android/app/src/androidTest -type f -name '*.kt' | sort)
  if [ -n "$violations" ]; then
    echo '回数の待ち・30 秒より短い上限が timing-ok: の理由なしに残っている:' >&2
    printf '%s' "$violations" | sed '/^$/d' >&2
    return 1
  fi
  echo "inventory: ${count} ファイルを一覧済み"
}

self_test() {
  local script out rc
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/ch09-inventory.XXXXXX")
  trap 'rm -rf -- "$tmp"' EXIT
  local change=openspec/changes/ch09-load-tolerant-timing-tests
  mkdir -p "$tmp/$change" "$tmp/crates/sample/src" "$tmp/web/e2e" "$tmp/web/src/__tests__" "$tmp/tools/smoke.d" \
    "$tmp/tools/recording" "$tmp/collector-android/app/src/test/k" "$tmp/collector-android/app/src/androidTest/k"
  : > "$tmp/web/playwright.config.ts"
  : > "$tmp/tools/noop.sh"
  : > "$tmp/tools/smoke.d/noop.sh"
  script="$tmp/$change/check-inventory.sh"
  cp "${BASH_SOURCE[0]}" "$script"
  chmod +x "$script"

  # expect_fail <説明> <出力に含むべき文言> [env の引数…]
  expect_fail() {
    local label=$1 want=$2
    shift 2
    rc=0
    out=$(cd "$tmp" && env "$@" bash "$script" 2>&1) || rc=$?
    if [ "$rc" -eq 0 ] || ! grep -Fq -- "$want" <<<"$out"; then
      printf '%s\n' "$out" >&2
      die "--self-test は ${label} を見逃した（rc=$rc、期待した文言: $want）"
    fi
  }

  echo "self-test: 候補 0 件"
  cat > "$tmp/$change/inventory.md" <<'EOF'
| 場所 | 型 | 方針 | 理由 |
|---|---|---|---|
EOF
  expect_fail '候補 0 件' '候補が 0 件' "PATH=$PATH"

  echo "self-test: 道具なし"
  mkdir -p "$tmp/nogrep"
  local tool
  for tool in bash dirname awk sort find sed wc cat env; do
    ln -s "$(command -v "$tool")" "$tmp/nogrep/$tool"
  done
  expect_fail '道具なし（grep）' '道具が無い: grep' "PATH=$tmp/nogrep"

  # 違反の例: for _ の条件の待ち（印あり・本体が 5 行を超える）、定数の眠り sleep(POLL)、_MS 定数、打ち切りの上限
  cat > "$tmp/crates/sample/src/sample_tests.rs" <<'EOF'
const POLL: std::time::Duration = std::time::Duration::from_millis(20);
async fn marked_but_counted() {
    // timing-ok: 誤った理由
    for _ in 0..40 {
        let a = 1;
        let b = 2;
        let c = 3;
        let d = a + b + c;
        if d > 0 && done() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let _ = std::time::Instant::now();
}
async fn const_sleep() {
    for _ in 0..500 {
        if waiting() {
            break;
        }
        tokio::time::sleep(POLL).await;
    }
}
EOF
  cat > "$tmp/collector-android/app/src/test/k/SampleTest.kt" <<'EOF'
class SampleTest {
    val started = System.nanoTime()
    companion object { const val SETTLE_MS = 5_000L }
    fun t() { assertTrue("打ち切り", tookMs < 3_000) }
}
EOF
  printf '%s\n' 'test("sample", async () => { await expect(x).toBeVisible({ timeout: 5_000 }); });' > "$tmp/web/e2e/sample.spec.ts"
  printf '%s\n' 'for _ in $(seq 1 2); do sleep 1; done' > "$tmp/tools/smoke.d/sample.sh"
  cat > "$tmp/$change/inventory.md" <<'EOF'
| 場所 | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/sample/src/sample_tests.rs:4` | 条件の待ち | 直した | self-test |
| `collector-android/app/src/test/k/SampleTest.kt:3` | 打ち切り | 直した | self-test |
| `web/e2e/sample.spec.ts:1` | 条件の待ち | 直した | self-test |
| tools/smoke.d/sample.sh | 条件の待ち | 直した | 裸のパスの self-test |
EOF
  echo "self-test: 印のある for _ の条件の待ち（5 行を超える本体）"
  expect_fail '印のある for _ の回数の待ち' 'sample_tests.rs:4: 回数で切る待ち' "PATH=$PATH"
  echo "self-test: sleep(POLL) の回数 × 眠り"
  expect_fail 'sleep(POLL)' 'sample_tests.rs:17: 回数で切る待ち（for _ in …、回数 × 眠り = 10.0 秒' "PATH=$PATH"
  echo "self-test: _MS 定数・打ち切りの上限"
  expect_fail '_MS 定数' '30 秒未満の _MS 定数（SETTLE_MS' "PATH=$PATH"
  expect_fail '打ち切りの上限' '30 秒未満の打ち切りの上限' "PATH=$PATH"
  echo "self-test: e2e と台本の短い待ち"
  expect_fail 'e2e の短い timeout' 'sample.spec.ts:1: Playwrightの30秒未満の期限' "PATH=$PATH"
  expect_fail '台本の回数×sleep' 'sample.sh:1: bashの回数×sleepが30秒未満' "PATH=$PATH"

  echo "self-test: 実在しないパスの行"
  cp "$tmp/$change/inventory.md" "$tmp/inventory.ok"
  printf '%s\n' '| `crates/sample/src/missing.rs:1` | 条件の待ち | 直した | self-test |' >> "$tmp/$change/inventory.md"
  expect_fail '実在しないパス' '実在しないパス: crates/sample/src/missing.rs' "PATH=$PATH"
  printf '%s\n' '| `observe_until` だけ | 条件の待ち | 直した | self-test |' >> "$tmp/$change/inventory.md"
  expect_fail 'パスの無い行' '場所の欄にパスが無い' "PATH=$PATH"
  cp "$tmp/inventory.ok" "$tmp/$change/inventory.md"

  # 直した形: 期限の待ち・時刻を兼ねる印つきのループ・30 秒以上の定数・印つきの窓
  cat > "$tmp/crates/sample/src/sample_tests.rs" <<'EOF'
async fn deadline_wait() {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    for _ in 0..40 {
        if done() || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}
fn clock_loop() {
    // timing-ok: t(sec) を進める時刻を兼ねるループ
    for sec in 0..10 {
        tick(t(sec));
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
async fn long_enough() {
    for _ in 0..3000 {
        if done() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
EOF
  cat > "$tmp/collector-android/app/src/test/k/SampleTest.kt" <<'EOF'
class SampleTest {
    val started = System.nanoTime()
    companion object {
        // timing-ok: 起きないことを見る窓
        const val SETTLE_MS = 5_000L
        const val POLL_MS = 250L
    }
    fun t() { assertTrue("打ち切り", tookMs < 30_000) }
}
EOF
  sed -i '1s/^/\/\/ timing-ok: self-test\n/' "$tmp/web/e2e/sample.spec.ts"
  sed -i '1s/^/# timing-ok: self-test\n/' "$tmp/tools/smoke.d/sample.sh"
  echo "self-test: ディレクトリの行はファイルに解決する"
  printf '%s\n' '| `crates/sample` | 条件の待ち | 直した | ディレクトリの self-test |' >> "$tmp/$change/inventory.md"
  (cd "$tmp" && bash "$script") >/dev/null || die '--self-test は直した形を落とした'
  cp "$tmp/$change/inventory.md" "$tmp/inventory.ok"
  printf '%s\n' '| `tools/smoke.d` | 条件の待ち | 直した | ディレクトリの self-test |' >> "$tmp/$change/inventory.md"
  printf '%s\n' 'for _ in $(seq 1 3); do sleep 1; done' > "$tmp/tools/smoke.d/other.sh"
  printf '%s\n' '| `tools/smoke.d/other.sh` | 固定の眠り | 直さない | 型の誤りの self-test |' >> "$tmp/$change/inventory.md"
  expect_fail 'ディレクトリの中の短い待ち' 'other.sh:1: bashの回数×sleepが30秒未満' "PATH=$PATH"
  rm "$tmp/tools/smoke.d/other.sh"
  cp "$tmp/inventory.ok" "$tmp/$change/inventory.md"
  (cd "$tmp" && bash "$script") >/dev/null || die '--self-test は直した形を落とした'
  echo 'check-inventory self-test: PASS'
}

case "${1:-}" in
  --list) list_candidates ;;
  --self-test) self_test ;;
  '') check_inventory ;;
  *) echo "使い方: $0 [--list|--self-test]" >&2; exit 2 ;;
esac
