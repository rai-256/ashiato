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
  local rev=$1 file=$2
  case "$file" in
    crates/*.rs)
      git cat-file -e "$rev:$file" 2>/dev/null || { echo 0; return; }
      git show "$rev:$file" | awk '
        /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/ { cfg = NR; next }
        cfg && /^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+tests([[:space:]]|$)/ {
          start = cfg
          cfg = 0
          next
        }
        { cfg = 0 }
        END { if (start) print start }
      '
      ;;
    *) echo 0 ;;
  esac
}

check_diff() {
  local base=$1 file old_start new_start line old_line new_line old_count new_count
  local bad=''
  while IFS= read -r file; do
    is_product_file "$file" || continue
    old_start=$(test_start "$base" "$file")
    new_start=$(test_start HEAD "$file")
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
          if ((new_line < new_start || new_start == 0)); then bad+="$file:$new_line: ${line:1}"$'\n'; fi
          new_line=$((new_line + 1))
          ;;
        -*)
          ((old_line > 0)) || continue
          if ((old_line < old_start || old_start == 0)); then bad+="$file:$old_line: ${line:1}"$'\n'; fi
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
  echo 'check-product-unchanged self-test: PASS'
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit $?; fi
[[ $# -eq 0 ]] || { echo "使い方: $0 [--self-test]" >&2; exit 2; }
base=$(git merge-base origin/main HEAD)
check_diff "$base"
echo "product: 変更なし（分岐点 ${base:0:8} → HEAD）"
