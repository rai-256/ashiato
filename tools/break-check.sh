#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "使い方: tools/break-check.sh --patch <差分> --expect <文言> [--expect <文言>…] [--timeout 秒] -- <命令…>" >&2
  echo "        tools/break-check.sh --self-test" >&2
}

die() {
  echo "NG: $*" >&2
  exit 1
}

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null) \
  || die "git リポジトリの中で実行してください"
CHECK_TEMP_ROOT=""
CHECK_WORKTREE=""
SELF_TEST_ROOT=""

run_check() {
  local patch_file="" timeout_seconds=900
  local -a expected=() command=()
  local arg

  while (($#)); do
    arg=$1
    case "$arg" in
      --patch)
        (($# >= 2)) || { usage; return 1; }
        patch_file=$2
        shift 2
        ;;
      --expect)
        (($# >= 2)) || { usage; return 1; }
        expected+=("$2")
        shift 2
        ;;
      --timeout)
        (($# >= 2)) || { usage; return 1; }
        timeout_seconds=$2
        shift 2
        ;;
      --)
        shift
        command=("$@")
        break
        ;;
      *)
        usage
        return 1
        ;;
    esac
  done

  [ -n "$patch_file" ] || { usage; return 1; }
  ((${#expected[@]} > 0)) || { usage; return 1; }
  ((${#command[@]} > 0)) || { usage; return 1; }
  [[ "$timeout_seconds" =~ ^[1-9][0-9]*$ ]] || die "timeout は正の整数で指定してください"
  [ -f "$patch_file" ] || die "差分が見つかりません: $patch_file"

  local patch_abs
  patch_abs=$(CDPATH= cd -- "$(dirname -- "$patch_file")" && pwd)/$(basename -- "$patch_file")

  local log command_rc=0
  CHECK_TEMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/break-check.XXXXXX")
  CHECK_WORKTREE=$CHECK_TEMP_ROOT/worktree
  log=$CHECK_TEMP_ROOT/command.log

  cleanup() {
    git -C "$REPO_ROOT" worktree remove --force "$CHECK_WORKTREE" >/dev/null 2>&1 || true
    git -C "$REPO_ROOT" worktree prune >/dev/null 2>&1 || true
    rm -rf -- "$CHECK_TEMP_ROOT"
  }
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  git -C "$REPO_ROOT" worktree add --detach "$CHECK_WORKTREE" HEAD >/dev/null \
    || die "使い捨ての作業木を作れません"
  if ! git -C "$CHECK_WORKTREE" apply "$patch_abs"; then
    echo "NG: 差分が当たらない"
    return 1
  fi

  echo "== 壊した作業木で命令を実行"
  if (cd "$CHECK_WORKTREE" && CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}" \
      timeout --signal=TERM --kill-after=2s "$timeout_seconds" "${command[@]}") \
      >"$log" 2>&1; then
    command_rc=0
  else
    command_rc=$?
  fi

  cat "$log"
  if [ "$command_rc" -eq 124 ]; then
    echo "NG: 上限で殺した"
    return 1
  fi
  if [ "$command_rc" -eq 0 ]; then
    echo "NG: 通った"
    return 1
  fi
  for arg in "${expected[@]}"; do
    if ! grep -Fq -- "$arg" "$log"; then
      echo "NG: 文言が違う（期待: $arg）"
      return 1
    fi
  done
  echo "OK: 壊した木で命令が期待どおり落ちた"
}

self_test() {
  local script="$SCRIPT_DIR/$(basename "$0")" before_status before_worktrees
  SELF_TEST_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/break-check-self-test.XXXXXX")
  trap 'rm -rf -- "$SELF_TEST_ROOT"' EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  (
    cd "$SELF_TEST_ROOT"
    git init -q -b main
    git config user.email test@example.invalid
    git config user.name break-check
    printf 'value=ok\n' > marker
    git add marker
    git commit -qm base
    cat > break.patch <<'PATCH'
diff --git a/marker b/marker
index 7f8c7c1..e75f9c1 100644
--- a/marker
+++ b/marker
@@ -1 +1 @@
-value=ok
+value=broken
PATCH
    cat > bad.patch <<'PATCH'
diff --git a/marker b/marker
index 7f8c7c1..e75f9c1 100644
--- a/marker
+++ b/marker
@@ -1 +1 @@
-value=not-present
+value=broken
PATCH
    before_status=$(git status --porcelain)
    before_worktrees=$(git worktree list --porcelain)

    echo "Scenario: 壊した木で命令が決まった文言で落ちれば、台本は通る"
    "$script" --patch break.patch --expect BROKEN -- bash -c \
      'grep -q value=broken marker && echo BROKEN && exit 1'

    echo "Scenario: 壊しても命令が通るか、文言が違えば、台本は落ちる"
    if output=$("$script" --patch break.patch --expect BROKEN -- bash -c 'echo OTHER; exit 0' 2>&1); then
      command_rc=0
    else
      command_rc=$?
    fi
    if [ "$command_rc" -ne 1 ] || ! grep -Fq 'NG: 通った' <<<"$output"; then
      echo "self-test NG: 通った命令を見逃した"
      printf '%s\n' "$output"
      exit 1
    fi
    if output=$("$script" --patch break.patch --expect BROKEN -- bash -c 'echo OTHER; exit 1' 2>&1); then
      command_rc=0
    else
      command_rc=$?
    fi
    if [ "$command_rc" -ne 1 ] || ! grep -Fq 'NG: 文言が違う（期待: BROKEN）' <<<"$output"; then
      echo "self-test NG: 文言違いを見逃した"
      printf '%s\n' "$output"
      exit 1
    fi
    if output=$("$script" --patch bad.patch --expect BROKEN -- bash -c 'exit 1' 2>&1); then
      command_rc=0
    else
      command_rc=$?
    fi
    if [ "$command_rc" -ne 1 ] || ! grep -Fq 'NG: 差分が当たらない' <<<"$output"; then
      echo "self-test NG: 当たらない差分を見逃した"
      printf '%s\n' "$output"
      exit 1
    fi

    echo "Scenario: 台本の後、元の作業木は変わらず、使い捨ての作業木は残らない"
    [ "$before_status" = "$(git status --porcelain)" ]
    [ "$before_worktrees" = "$(git worktree list --porcelain)" ]

    echo "Scenario: 上限より長くかかる命令は殺され、台本は落ちる"
    started_at=$SECONDS
    if output=$("$script" --patch break.patch --expect NEVER --timeout 2 -- bash -c 'sleep 60' 2>&1); then
      command_rc=0
    else
      command_rc=$?
    fi
    elapsed=$((SECONDS - started_at))
    if [ "$command_rc" -ne 1 ] || [ "$elapsed" -lt 1 ] || [ "$elapsed" -ge 10 ] || \
      ! grep -Fq 'NG: 上限で殺した' <<<"$output"; then
      echo "self-test NG: timeout を見逃した"
      printf 'rc=%s elapsed=%ss\n%s\n' "$command_rc" "$elapsed" "$output"
      exit 1
    fi
  )
}

if [ "${1:-}" = "--self-test" ]; then
  [ "$#" -eq 1 ] || { usage; exit 1; }
  self_test
else
  run_check "$@"
fi
