#!/usr/bin/env bash
# 試験が一時フォルダに残したものを測る（CH03 テストの一時ファイルの後始末）。
#
#   tools/tmp-leak-check.sh rust [cargo test の引数…]     # 引数が無ければ --workspace
#   tools/tmp-leak-check.sh android [gradle の引数…]      # :app:testDebugUnitTest --rerun に足す
#   tools/tmp-leak-check.sh --self-test                   # 偽の命令で、残り・0 本・rc の伝わり・通る道を見る
#
# rc: 試験の rc が 0 でなければその rc / 走った試験が 0 本なら 1 / 根に何か残れば 1 / 通れば 0。
#     rc=0 のときは根を消す。rc=1 のときは残す（中身を見て直す。`target/` の下なので /tmp を食わない）。
# 試験の命令は環境変数 TMP_LEAK_CMD で差し替えられる（--self-test が使う。本数は出力の
#     `test result: ok. <N> passed` の N の合計。差し替え中の命令には根が TMP_LEAK_ROOT で渡る）。
#
# **なぜ要るか**: 試験が自分の一時フォルダを消し忘れても、`/tmp` に溜まるだけで何も落ちなかった（CH03）。
# 一時フォルダを worktree の中の空のフォルダへ向けて走らせ、終わった後に何か残っていれば落とす。
# Rust は TMPDIR、Kotlin は ASHIATO_TEST_TMPDIR（build.gradle.kts が試験の JVM の java.io.tmpdir に渡す）。
set -uo pipefail
cd "$(dirname "$0")/.."
repo=$PWD

count_rust_output() { # stdin の `test result: ok. N passed` の N の合計
  { grep -oE 'test result: ok\. [0-9]+ passed' || true; } | awk '{s+=$4} END{print s+0}'
}

count_android_xml() {
  local d=collector-android/app/build/test-results/testDebugUnitTest f
  for f in "$d"/*.xml; do
    [ -f "$f" ] && grep -m1 -oE '<testsuite [^>]*' "$f" | grep -oE ' tests="[0-9]+"' | grep -oE '[0-9]+'
  done | awk '{s+=$1} END{print s+0}'
}

run() {
  local mode=$1; shift
  local stamp root out rc ran
  stamp=$(date +%Y%m%d-%H%M%S)
  root="$repo/target/tmp-leak/$mode-$stamp-$$"
  mkdir -p "$root"
  [ -z "$(ls -A "$root")" ] || { echo "tmp-leak: 根が空でない: $root" >&2; return 2; }
  echo "tmp-leak: 根 $root"
  out=$(mktemp "$repo/target/tmp-leak/out.XXXXXX")

  if [ -n "${TMP_LEAK_CMD:-}" ]; then
    TMPDIR=$root ASHIATO_TEST_TMPDIR=$root TMP_LEAK_ROOT=$root bash -c "$TMP_LEAK_CMD" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    ran=$(count_rust_output <"$out")
  elif [ "$mode" = rust ]; then
    [ $# -gt 0 ] || set -- --workspace
    cargo test --no-run "$@" || { rc=$?; rm -f "$out"; return "$rc"; }
    TMPDIR=$root cargo test "$@" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    ran=$(count_rust_output <"$out")
  else
    ASHIATO_TEST_TMPDIR=$root ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --rerun "$@" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    ran=$(count_android_xml)
  fi
  rm -f "$out"

  if [ "$rc" -ne 0 ]; then
    echo "tmp-leak: 試験が rc=$rc で終わった（根は残す: $root）" >&2
    return "$rc"
  fi
  if [ "$ran" -eq 0 ]; then
    echo "tmp-leak: 走った試験が 0 本（根は残す: $root）" >&2
    return 1
  fi
  local left
  left=$(ls -A "$root" | wc -l)
  if [ "$left" -gt 0 ]; then
    echo "tmp-leak: 試験の後に一時フォルダへ残ったものが $left 件ある（根: $root）" >&2
    ls -A "$root" | head -20 | sed 's/^/  残り: /' >&2
    return 1
  fi
  rm -rf "$root"
  echo "tmp-leak: OK 試験 $ran 本、残り無し"
}

self_test() {
  local fails=0 out rc root
  expect() { # 名前 期待rc 期待の語 命令
    local name=$1 want_rc=$2 want_text=$3 cmd=$4
    echo "Scenario: $name"
    out=$(TMP_LEAK_CMD=$cmd "$0" rust 2>&1); rc=$?
    root=$(printf '%s\n' "$out" | sed -n 's/^tmp-leak: 根 //p' | head -1)
    if [ "$rc" -ne "$want_rc" ] || ! printf '%s' "$out" | grep -qF -- "$want_text"; then
      echo "FAIL: $name (rc=$rc, 期待 rc=$want_rc と「$want_text」)" >&2
      printf '%s\n' "$out" >&2
      fails=$((fails + 1))
    fi
    [ -n "$root" ] && rm -rf "$root"
  }
  expect "試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる" 1 "leaked-by-test" \
    'touch "$TMP_LEAK_ROOT/leaked-by-test"; echo "test result: ok. 2 passed; 0 failed"'
  expect "試験が 1 本も走らなければ、一時フォルダが空でも落ちる" 1 "0 本" \
    'echo "test result: ok. 0 passed; 0 failed"'
  expect "試験が落ちれば、台本も試験の終了コードで落ちる" 101 "rc=101" \
    'echo "test result: FAILED. 0 passed; 1 failed"; exit 101'
  expect "試験が通り、何も残さなければ、台本は通る" 0 "試験 3 本" \
    'echo "test result: ok. 3 passed; 0 failed"'
  echo "Scenario: CI の試験の段は、残りを測る台本から走る"
  local ci=.github/workflows/ci.yml
  if grep -nE '^\s*-?\s*run:\s*(\./)?(cargo test --workspace|gradlew :app:testDebugUnitTest)\s*$' "$ci"; then
    echo "FAIL: $ci に裸の試験の段がある（tmp-leak-check.sh から走らせる）" >&2
    fails=$((fails + 1))
  fi
  for want in 'tmp-leak-check.sh rust' 'tmp-leak-check.sh android'; do
    grep -qE "run:.*$want\s*$" "$ci" || { echo "FAIL: $ci に「$want」の段が無い" >&2; fails=$((fails + 1)); }
  done
  [ "$fails" -eq 0 ] && echo "tmp-leak: self-test OK"
  [ "$fails" -eq 0 ]
}

case "${1:-}" in
  --self-test) self_test ;;
  rust | android) mode=$1; shift; mkdir -p target/tmp-leak; run "$mode" "$@" ;;
  *) echo "使い方: $0 rust [引数…] | android [引数…] | --self-test" >&2; exit 2 ;;
esac
