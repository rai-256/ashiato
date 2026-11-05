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

test_start() {
  local file=$1
  case "$file" in
    crates/*.rs)
      awk '/^[[:space:]]*#\[cfg\(test\)\]/{print NR; exit}' "$file"
      ;;
    *) echo 0 ;;
  esac
}

check_diff() {
  local base=$1 file start line old_line new_line old_count new_count
  local bad='' old_line=0 new_line=0
  while IFS= read -r file; do
    is_product_file "$file" || continue
    start=$(test_start "$file")
    while IFS= read -r line; do
      if [[ "$line" =~ ^@@[[:space:]]-([0-9]+)(,([0-9]+))?[[:space:]]\+([0-9]+)(,([0-9]+))? ]]; then
        old_line=${BASH_REMATCH[1]}
        new_line=${BASH_REMATCH[4]}
        old_count=${BASH_REMATCH[3]:-1}
        new_count=${BASH_REMATCH[6]:-1}
        continue
      fi
      [[ "$line" == ++++* || "$line" == ---* ]] && continue
      case "$line" in
        +*)
          ((new_line > 0)) || continue
          if ((new_line < start || start == 0)); then bad+="$file:$new_line: ${line:1}"$'\n'; fi
          new_line=$((new_line + 1))
          ;;
        -*)
          ((old_line > 0)) || continue
          if ((old_line < start || start == 0)); then bad+="$file:$old_line: ${line:1}"$'\n'; fi
          old_line=$((old_line + 1))
          ;;
        ' '*)
          old_line=$((old_line + 1)); new_line=$((new_line + 1))
          ;;
      esac
    done < <(git diff --unified=0 "$base" -- "$file")
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
  printf '%s\n' '#[cfg(test)]' 'mod tests {}' >> "$tmp/repo/crates/app/lib.rs"
  printf '%s\n' 'export const product = 1;' > "$tmp/repo/web/src/app.ts"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm base
  base=$(git -C "$tmp/repo" rev-parse HEAD)
  git -C "$tmp/repo" branch main "$base"
  git -C "$tmp/repo" update-ref refs/remotes/origin/main "$base"
  git -C "$tmp/repo" switch -qc work
  printf '%s\n' 'fn changed_product() {}' > "$tmp/repo/crates/app/lib.rs"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm product
  head=$(git -C "$tmp/repo" rev-parse HEAD)
  mkdir -p "$tmp/repo/openspec/changes/ch09-load-tolerant-timing-tests"
  cp "${BASH_SOURCE[0]}" "$tmp/repo/openspec/changes/ch09-load-tolerant-timing-tests/check.sh"
  if (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null 2>&1; then
    echo 'check-product-unchanged self-test: 製品変更を見逃した' >&2
    return 1
  fi
  git -C "$tmp/repo" checkout -q "$base" -- crates/app/lib.rs
  printf '%s\n' 'fn product() {}' '#[cfg(test)]' 'mod tests { fn added() {} }' > "$tmp/repo/crates/app/lib.rs"
  git -C "$tmp/repo" add . && git -C "$tmp/repo" commit -qm tests
  (cd "$tmp/repo" && openspec/changes/ch09-load-tolerant-timing-tests/check.sh) >/dev/null
  echo 'check-product-unchanged self-test: PASS'
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit $?; fi
[[ $# -eq 0 ]] || { echo "使い方: $0 [--self-test]" >&2; exit 2; }
base=$(git merge-base origin/main HEAD)
check_diff "$base"
echo "product: 変更なし（分岐点 ${base:0:8} → HEAD）"
