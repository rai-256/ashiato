#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$ROOT"

echo "Scenario: CH09 の差分は、試験のモジュールの外の製品のコードに 0 行"

is_product_file() {
  case "$1" in
    crates/*.rs)
      case "$1" in
        *_tests.rs|*/testdb.rs|*/tests/*) return 1 ;;
        *) return 0 ;;
      esac
      ;;
    collector-android/app/src/main/*|web/src/*)
      [[ "$1" != web/src/__tests__/* ]]
      ;;
    *) return 1 ;;
  esac
}

# 試験のモジュール（`#[cfg(test)]` + `mod tests {`）の開始行と閉じの行を「開始 終わり」で出す。無ければ「0 0」。
# 閉じは波括弧の対応で求める（文字列・文字・行の注記の中の波括弧は数えない）。開始の行から後ろを全部
# 試験とみなすと、試験モジュールの後ろに足した製品のコードを通す（review/code.md R14）。
test_range() {
  local rev=$1 file=$2
  case "$file" in
    crates/*.rs)
      git cat-file -e "$rev:$file" 2>/dev/null || { echo '0 0'; return; }
      git show "$rev:$file" | awk -v file="$rev:$file" '
        function strip(line,    out, i, c, n) {
          out = ""; n = length(line)
          for (i = 1; i <= n; i++) {
            c = substr(line, i, 1)
            if (in_str) {
              if (c == "\\") { i++; continue }
              if (c == "\"") in_str = 0
              continue
            }
            if (c == "/" && substr(line, i + 1, 1) == "/") break
            if (c == "\"") { in_str = 1; continue }
            if (c == "'"'"'" && substr(line, i + 2, 1) == "'"'"'") { i += 2; continue }
            if (c == "'"'"'" && substr(line, i + 1, 1) == "\\" && substr(line, i + 3, 1) == "'"'"'") { i += 3; continue }
            out = out c
          }
          return out
        }
        start && !end {
          line = strip($0)
          depth += gsub(/\{/, "{", line) - gsub(/\}/, "}", line)
          if (depth <= 0 && opened) end = NR
          if (line ~ /\{/) opened = 1
          if (depth <= 0 && opened) end = NR
          next
        }
        /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/ { cfg = NR; next }
        cfg && !start && /^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+tests([[:space:]]|$)/ {
          start = cfg
          line = strip($0)
          depth = gsub(/\{/, "{", line) - gsub(/\}/, "}", line)
          opened = (line ~ /\{/)
          if (opened && depth <= 0) end = NR
          cfg = 0
          next
        }
        { cfg = 0 }
        END {
          if (!start) { print "0 0"; exit }
          if (!end) { printf "check-product-unchanged: %s の試験モジュールの閉じが見つからない（%d 行目から）\n", file, start > "/dev/stderr"; exit 1 }
          print start, end
        }
      '
      ;;
    *) echo '0 0' ;;
  esac
}

# 行が製品か（試験モジュールの範囲の外か）
is_product_line() {
  local n=$1 start=$2 end=$3
  ((start == 0 || n < start || n > end))
}

check_diff() {
  local base=$1 file old_start old_end new_start new_end line old_line new_line old_count new_count
  local bad=''
  while IFS= read -r file; do
    is_product_file "$file" || continue
    read -r old_start old_end < <(test_range "$base" "$file") || { echo "check-product-unchanged: $file の試験モジュールを読めない" >&2; return 1; }
    read -r new_start new_end < <(test_range HEAD "$file") || { echo "check-product-unchanged: $file の試験モジュールを読めない" >&2; return 1; }
    [[ "$old_start" =~ ^[0-9]+$ && "$new_start" =~ ^[0-9]+$ ]] || { echo "check-product-unchanged: $file の試験モジュールを読めない" >&2; return 1; }
    old_line=0
    new_line=0
    while IFS= read -r line; do
      if [[ "$line" =~ ^@@[[:space:]]-([0-9]+)(,([0-9]+))?[[:space:]]\+([0-9]+)(,([0-9]+))? ]]; then
        old_line=${BASH_REMATCH[1]}
        new_line=${BASH_REMATCH[4]}
        old_count=${BASH_REMATCH[3]:-1}
        new_count=${BASH_REMATCH[6]:-1}
        continue
      fi
      [[ "$line" == '+++ '* || "$line" == '--- '* ]] && continue
      case "$line" in
          +*)
          ((new_line > 0)) || continue
          if is_product_line "$new_line" "$new_start" "$new_end"; then bad+="$file:$new_line: ${line:1}"$'\n'; fi
          new_line=$((new_line + 1))
          ;;
        -*)
          ((old_line > 0)) || continue
          if is_product_line "$old_line" "$old_start" "$old_end"; then bad+="$file:$old_line: ${line:1}"$'\n'; fi
          old_line=$((old_line + 1))
          ;;
        ' '*)
          old_line=$((old_line + 1)); new_line=$((new_line + 1))
          ;;
      esac
    done < <(git diff --unified=0 "$base" HEAD -- "$file")
  done < <(git diff --name-only "$base" HEAD -- crates collector-android/app/src/main web/src)
  if [[ -n "$bad" ]]; then
    echo '製品コードの変更:' >&2
    printf '%s' "$bad" >&2
    return 1
  fi
}

self_test_cleanup=''
self_test() {
  local tmp base
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/ch09-product-check.XXXXXX")
  self_test_cleanup=$tmp
  trap 'rm -rf -- "$self_test_cleanup"' EXIT
  git init -q "$tmp/repo"
  git -C "$tmp/repo" config user.email test@example.invalid
  git -C "$tmp/repo" config user.name test
  mkdir -p "$tmp/repo/crates/app" "$tmp/repo/web/src/__tests__"
  printf '%s\n' 'fn product() {}' > "$tmp/repo/crates/app/lib.rs"
  printf '%s\n' '#[cfg(test)]' 'const TEST_HELPER: bool = true;' '#[cfg(test)]' 'mod tests {}' >> "$tmp/repo/crates/app/lib.rs"
  printf '%s\n' 'export const product = 1;' > "$tmp/repo/web/src/app.ts"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm base
  base=$(git -C "$tmp/repo" rev-parse HEAD)
  git -C "$tmp/repo" branch main "$base"
  git -C "$tmp/repo" update-ref refs/remotes/origin/main "$base"
  git -C "$tmp/repo" switch -qc work
  printf '%s\n' 'fn changed_product() {}' '#[cfg(test)]' 'const TEST_HELPER: bool = false;' '#[cfg(test)]' 'mod tests {}' > "$tmp/repo/crates/app/lib.rs"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm product
  head=$(git -C "$tmp/repo" rev-parse HEAD)
  mkdir -p "$tmp/repo/openspec/changes/ch09-load-tolerant-timing-tests"
  cp "${BASH_SOURCE[0]}" "$tmp/repo/openspec/changes/ch09-load-tolerant-timing-tests/check.sh"
  if (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null 2>&1; then
    echo 'check-product-unchanged self-test: 製品変更を見逃した' >&2
    return 1
  fi
  git -C "$tmp/repo" checkout -q "$base" -- crates/app/lib.rs
  printf '%s\n' 'fn product() {}' '#[cfg(test)]' 'const TEST_HELPER: bool = true;' '#[cfg(test)]' 'mod tests { fn added() {} }' > "$tmp/repo/crates/app/lib.rs"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm tests
  (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null

  printf '%s\n' 'fn product() {}' '#[cfg(test)]' 'const TEST_HELPER: bool = true;' '#[cfg(test)]' 'mod tests { fn added() {} }' > "$tmp/repo/crates/app/lib.rs"
  printf '%s\n' 'export const changed_product = 2;' > "$tmp/repo/web/src/app.ts"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm product_again
  if (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null 2>&1; then
    echo 'check-product-unchanged self-test: 複数ファイルの製品変更を見逃した' >&2
    return 1
  fi
  git -C "$tmp/repo" checkout -q "$base" -- web/src/app.ts
  printf '%s\n' 'fn product() {}' '#[cfg(test)]' 'const TEST_HELPER: bool = true;' '#[cfg(test)]' 'mod tests {' '    fn added() { let _ = "}"; }' '}' 'pub fn sneaky_product() -> u32 { 42 }' > "$tmp/repo/crates/app/lib.rs"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm after_tests
  if (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null 2>&1; then
    echo 'check-product-unchanged self-test: 試験モジュールの後ろの製品変更を見逃した' >&2
    return 1
  fi
  echo 'check-product-unchanged self-test: PASS'
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit $?; fi
[[ $# -eq 0 ]] || { echo "使い方: $0 [--self-test]" >&2; exit 2; }
base=$(git merge-base origin/main HEAD)
check_diff "$base"
echo "product: 変更なし（分岐点 ${base:0:8} → HEAD）"
