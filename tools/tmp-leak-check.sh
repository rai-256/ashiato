#!/usr/bin/env bash
# 試験が一時フォルダに残したものを測る（CH03 テストの一時ファイルの後始末）。
#
#   tools/tmp-leak-check.sh rust [cargo test の引数…]     # 引数が無ければ --workspace
#   tools/tmp-leak-check.sh android [gradle の引数…]      # :app:testDebugUnitTest --rerun に足す
#   tools/tmp-leak-check.sh --self-test                   # 偽の命令で、残り・0 本・rc の伝わり・通る道を見る
#
# rc: 試験の rc が 0 でなければその rc / 走った試験が 0 本なら 1 / 根に何か残れば 1 / 通れば 0。
#     rc=0 のときは根を消す。落ちたときは根を `target/tmp-leak/` へ移して残す（中身を見て直す。/tmp を食わない）。
# 根は /tmp（tmpfs）に作る: ディスクの上だと、置き場に書く試験が 10 倍以上遅くなり、時間を測る試験が落ちる
#     （2026-10-27 実測: collector-windows の 283 本が /tmp で 5.7 秒、target/ で 63〜90 秒。
#     history_large_fetch_keeps_observing_foreground が見回り 1 回 1.1〜2.2 秒で毎回落ちた）。
#     作る場所は TMP_LEAK_BASE、移す先は TMP_LEAK_KEEP で差し替えられる（--self-test が使う）。
# 試験の命令は環境変数 TMP_LEAK_CMD で差し替えられる（--self-test が使う。差し替え中の命令には根が TMP_LEAK_ROOT で渡る）。
#     本数は rust なら出力の `test result: ok. <N> passed` の N の合計、android なら JUnit XML の
#     `tests` から `skipped` を引いた合計（全部が飛ばされた回を 0 本として落とす）。
#     XML の置き場は TMP_LEAK_XML_DIR で差し替えられる（--self-test が偽の XML を置く）。
#
# **なぜ要るか**: 試験が自分の一時フォルダを消し忘れても、`/tmp` に溜まるだけで何も落ちなかった（CH03）。
# 一時フォルダを空のフォルダへ向けて走らせ、終わった後に何か残っていれば落とす。
# Rust は TMPDIR、Kotlin は ASHIATO_TEST_TMPDIR（build.gradle.kts が試験の JVM の java.io.tmpdir に渡す）。
set -uo pipefail
cd "$(dirname "$0")/.."
repo=$PWD
base=${TMP_LEAK_BASE:-/tmp}
keep=${TMP_LEAK_KEEP:-$repo/target/tmp-leak}
xml_dir=${TMP_LEAK_XML_DIR:-$repo/collector-android/app/build/test-results/testDebugUnitTest}

count_rust_output() { # stdin の `test result: ok. N passed` の N の合計
  { grep -oE 'test result: ok\. [0-9]+ passed' || true; } | awk '{s+=$4} END{print s+0}'
}

attr() { # 属性名 —— stdin の要素の頭から 属性名="N" の N（無ければ 0）
  { grep -oE " $1=\"[0-9]+\"" || echo '="0"'; } | head -1 | grep -oE '[0-9]+'
}

count_android_xml() { # 走った本数 = tests - skipped（飛ばされただけの試験は走っていない）
  local d=$xml_dir f head
  for f in "$d"/*.xml; do
    [ -f "$f" ] || continue
    head=$(grep -m1 -oE '<testsuite [^>]*' "$f") || continue
    echo "$(attr tests <<<"$head") $(attr skipped <<<"$head")"
  done | awk '{s+=$1-$2} END{print s+0}'
}

keep_root() { # 落ちた回の根を /tmp から target/tmp-leak/ へ移し、移した先を出す
  local to
  to="$keep/$(basename "$1")"
  if mv "$1" "$to" 2>/dev/null; then echo "$to"; else echo "$1（target へ移せなかった）"; fi
}

run() {
  local mode=$1; shift
  local stamp root out rc ran
  stamp=$(date +%Y%m%d-%H%M%S)
  root="$base/ashiato-tmp-leak-$mode-$stamp-$$"
  mkdir -p "$root"
  [ -z "$(ls -A "$root")" ] || { echo "tmp-leak: 根が空でない: $root" >&2; return 2; }
  echo "tmp-leak: 根 $root"
  out=$(mktemp "$keep/out.XXXXXX")

  if [ -n "${TMP_LEAK_CMD:-}" ]; then
    TMPDIR=$root ASHIATO_TEST_TMPDIR=$root TMP_LEAK_ROOT=$root bash -c "$TMP_LEAK_CMD" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    if [ "$mode" = android ]; then ran=$(count_android_xml); else ran=$(count_rust_output <"$out"); fi
  elif [ "$mode" = rust ]; then
    [ $# -gt 0 ] || set -- --workspace
    echo "Scenario: 一時フォルダを空のフォルダへ向けて試験を一通り走らせると、終わった後も空のまま"
    cargo test --no-run "$@" || { rc=$?; rm -f "$out"; rm -rf "$root"; return "$rc"; }
    TMPDIR=$root cargo test "$@" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    ran=$(count_rust_output <"$out")
  else
    echo "Scenario: 一時フォルダを空のフォルダへ向けて試験を一通り走らせると、終わった後も空のまま"
    ASHIATO_TEST_TMPDIR=$root ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --rerun "$@" 2>&1 | tee "$out"
    rc=${PIPESTATUS[0]}
    ran=$(count_android_xml)
  fi
  rm -f "$out"

  if [ "$rc" -ne 0 ]; then
    echo "tmp-leak: 試験が rc=$rc で終わった（根は残す: $(keep_root "$root")）" >&2
    return "$rc"
  fi
  if [ "$ran" -eq 0 ]; then
    echo "tmp-leak: 走った試験が 0 本（根は残す: $(keep_root "$root")）" >&2
    return 1
  fi
  local left
  left=$(ls -A "$root" | wc -l)
  if [ "$left" -gt 0 ]; then
    ls -A "$root" | head -20 | sed 's/^/  残り: /' >&2
    echo "tmp-leak: 試験の後に一時フォルダへ残ったものが $left 件ある（根: $(keep_root "$root")）" >&2
    return 1
  fi
  rm -rf "$root"
  echo "tmp-leak: OK 試験 $ran 本、残り無し"
}

self_test() {
  local fails=0 out rc kept xml
  kept=$(mktemp -d)
  xml=$(mktemp -d)
  trap 'rm -rf "$kept" "$xml"' RETURN
  expect() { # 名前 期待rc 期待の語 命令 [rust|android]
    local name=$1 want_rc=$2 want_text=$3 cmd=$4 mode=${5:-rust}
    echo "Scenario: $name"
    out=$(TMP_LEAK_CMD=$cmd TMP_LEAK_KEEP=$kept TMP_LEAK_XML_DIR=$xml "$0" "$mode" 2>&1); rc=$?
    if [ "$rc" -ne "$want_rc" ] || ! printf '%s' "$out" | grep -qF -- "$want_text"; then
      echo "FAIL: $name (rc=$rc, 期待 rc=$want_rc と「$want_text」)" >&2
      printf '%s\n' "$out" >&2
      fails=$((fails + 1))
    fi
  }
  # Scenario: 試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる
  expect "試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる" 1 "leaked-by-test" \
    'touch "$TMP_LEAK_ROOT/leaked-by-test"; echo "test result: ok. 2 passed; 0 failed"'
  # Scenario: 試験が 1 本も走らなければ、一時フォルダが空でも落ちる
  expect "試験が 1 本も走らなければ、一時フォルダが空でも落ちる" 1 "0 本" \
    'echo "test result: ok. 0 passed; 0 failed"'
  # Scenario: 試験が落ちれば、台本も試験の終了コードで落ちる
  expect "試験が落ちれば、台本も試験の終了コードで落ちる" 101 "rc=101" \
    'echo "test result: FAILED. 0 passed; 1 failed"; exit 101'
  # Scenario: 試験が通り、何も残さなければ、台本は通る
  expect "試験が通り、何も残さなければ、台本は通る" 0 "試験 3 本" \
    'echo "test result: ok. 3 passed; 0 failed"'
  # android の道（JUnit XML の本数）。偽の XML は命令が置く（Gradle が結果のフォルダを空にしてから書くのと同じ）
  local suite='rm -f "$TMP_LEAK_XML_DIR"/*.xml; w() { printf "<?xml version=\"1.0\"?>\n<testsuite name=\"%s\" tests=\"%s\" skipped=\"%s\" failures=\"0\" errors=\"0\">\n</testsuite>\n" "$1" "$2" "$3" >"$TMP_LEAK_XML_DIR/TEST-$1.xml"; }; '
  expect "試験が 1 本も走らなければ、一時フォルダが空でも落ちる（android: 全部が飛ばされた）" 1 "0 本" \
    "$suite"'w a.SkippedTest 2 2; w b.SkippedTest 1 1' android
  expect "試験が通り、何も残さなければ、台本は通る（android: 飛ばした分を引く）" 0 "試験 4 本" \
    "$suite"'w a.ProbeTest 3 1; w b.OtherTest 2 0; w c.NoSkipAttr 0 0' android
  expect "試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる（android）" 1 "leaked-by-robolectric" \
    "$suite"'w a.ProbeTest 1 0; touch "$TMP_LEAK_ROOT/leaked-by-robolectric"' android
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
  rust | android) mode=$1; shift; mkdir -p "$keep"; run "$mode" "$@" ;;
  *) echo "使い方: $0 rust [引数…] | android [引数…] | --self-test" >&2; exit 2 ;;
esac
