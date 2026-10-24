#!/usr/bin/env bash
# 試験が database を作る書き方の検査（CH04 / design D5）。
#
#   ./tools/check-test-db-growth.sh --lint       → 本物の木を検査する（rc=0 通過 / rc=1 違反 / rc=2 使い方）
#   ./tools/check-test-db-growth.sh --self-test  → 偽の木で、落ちるべきものが落ち、通すべきものが通ることを確かめる
#
# 見るのは 2 つ（どちらも `crates/server` の下）:
#   1. `src` と `tests` の `CREATE DATABASE` —— `testdb.rs` の中だけに書く。ほかは `testdb::isolated()`（回のテンプレートの写し）か
#      `testdb::scratch()`（空）を使う。
#   2. `src/export*_tests.rs` の `testdb::pool()` —— 書き出しの試験は回の database を共有せず、試験ごとの database（`testdb::isolated()`）を取る。
# 通すのは `testdb.rs` と、同じ行か 1 行上に理由つきの `db-ok: <理由>` がある行。
#
# なぜ要るか（CH04）: 試験の database の作り方が 5 か所に散り、回の印を持たない `st*_tmp_…` が `ashiato` の隣に溜まって
# 試験が遅くなった。走っている Story が新しく足す所も、この検査が rebase の後に止める。
set -euo pipefail
cd "$(dirname "$0")/.."

# lint <木の根> : 違反を「ファイル:行: 内容」で出し、あれば rc=1
lint() {
  local root=$1 bad=0 hits
  # 1 つ目: CREATE DATABASE。`file:line:text` を挙げ、同じ行か 1 行上の db-ok: で通す
  hits=$(cd "$root" && grep -rnE 'CREATE DATABASE' crates/server/src crates/server/tests --include='*.rs' 2>/dev/null \
    | grep -v '^crates/server/src/testdb\.rs:' || true)
  # 行頭が // の説明の行（/// も）は、書き方でなく文章なので見ない
  hits=$(grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' <<<"$hits" || true)
  report "$root" "$hits" "CREATE DATABASE は testdb.rs の中だけに書く。試験ごとの写しは testdb::isolated()、空の使い捨ては testdb::scratch() を使う。持ち主のいない回の database を作る片付けの試験などは、同じ行か 1 行上に db-ok: <理由> を書く" || bad=1
  # 2 つ目: 書き出しの試験の testdb::pool()
  hits=$(cd "$root" && grep -HnE 'testdb::pool\(\)' crates/server/src/export*_tests.rs 2>/dev/null || true)
  report "$root" "$hits" "書き出しの試験は回の database を共有しない。試験ごとの database を取る testdb::isolated() を使う（回の database を見ること自体を確かめるなら、同じ行か 1 行上に db-ok: <理由> を書く）" || bad=1
  return $bad
}

# report <根> <file:line:text の列> <落ちたときの文言> : db-ok: の無い行だけ出す。あれば rc=1
report() {
  local root=$1 hits=$2 msg=$3 bad=0 file line text prev
  [[ -z $hits ]] && return 0
  while IFS=: read -r file line text; do
    [[ -z $file ]] && continue
    prev=""
    (( line > 1 )) && prev=$(sed -n "$((line - 1))p" "$root/$file")
    if grep -qE 'db-ok: *[^ ]' <<<"$text" || grep -qE 'db-ok: *[^ ]' <<<"$prev"; then continue; fi
    echo "$file:$line: $text" >&2
    bad=1
  done <<<"$hits"
  [[ $bad == 1 ]] && echo "  → $msg" >&2
  return $bad
}

self_test() {
  local fail=0 out rc
  tmp=$(mktemp -d)  # trap が関数の外で走るので大域
  trap 'rm -rf "$tmp"' EXIT
  fake() { rm -rf "$tmp/t"; mkdir -p "$tmp/t/crates/server/src" "$tmp/t/crates/server/tests"; }
  want() {  # want <説明> <条件の rc>
    if [[ $2 == 0 ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1" >&2; fail=1; fi
  }
  run() { set +e; out=$(lint "$tmp/t" 2>&1); rc=$?; set -e; }

  echo "Scenario: 足場の外で database を作る試験は、検査が落とし、足場の関数を示す"
  fake
  printf 'async fn f() {\n    sqlx::raw_sql("CREATE DATABASE x");\n}\n' >"$tmp/t/crates/server/src/x_tests.rs"
  printf 'sqlx::raw_sql("CREATE DATABASE inside_testdb");\n' >"$tmp/t/crates/server/src/testdb.rs"
  run
  want "x_tests.rs の CREATE DATABASE で rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "ファイルと行を出す" "$(grep -q 'crates/server/src/x_tests.rs:2:' <<<"$out"; echo $?)"
  want "testdb::scratch を示す" "$(grep -q 'testdb::scratch' <<<"$out"; echo $?)"
  want "testdb.rs の中は通す" "$(grep -q 'testdb.rs:' <<<"$out"; [[ $? == 1 ]]; echo $?)"
  printf 'async fn f() {\n    // db-ok: 片付けの試験の材料\n    sqlx::raw_sql("CREATE DATABASE x");\n}\n' >"$tmp/t/crates/server/src/x_tests.rs"
  run
  want "1 行上の理由つき db-ok: で rc=0" "$([[ $rc == 0 ]]; echo $?)"
  printf 'sqlx::raw_sql("CREATE DATABASE x"); // db-ok: 材料\n' >"$tmp/t/crates/server/src/x_tests.rs"
  run
  want "同じ行の理由つき db-ok: で rc=0" "$([[ $rc == 0 ]]; echo $?)"
  printf '// db-ok:\nsqlx::raw_sql("CREATE DATABASE x");\n' >"$tmp/t/crates/server/src/x_tests.rs"
  run
  want "理由の無い db-ok: は通さない" "$([[ $rc == 1 ]]; echo $?)"

  echo "Scenario: 書き出しの試験が回の database を取ると、検査が落とし、試験ごとの database を示す"
  fake
  printf 'async fn f() {\n    let p = testdb::pool().await;\n}\n' >"$tmp/t/crates/server/src/export_x_tests.rs"
  run
  want "export_x_tests.rs の testdb::pool() で rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "ファイルと行を出す" "$(grep -q 'crates/server/src/export_x_tests.rs:2:' <<<"$out"; echo $?)"
  want "testdb::isolated を示す" "$(grep -q 'testdb::isolated' <<<"$out"; echo $?)"
  printf 'async fn f() {\n    // db-ok: 回の database を見ること自体を確かめる\n    let p = testdb::pool().await;\n}\n' >"$tmp/t/crates/server/src/export_x_tests.rs"
  run
  want "1 行上の理由つき db-ok: で rc=0" "$([[ $rc == 0 ]]; echo $?)"
  printf 'let p = testdb::pool().await;\n' >"$tmp/t/crates/server/src/other_tests.rs"
  rm "$tmp/t/crates/server/src/export_x_tests.rs"
  run
  want "export 以外の試験の testdb::pool() は見ない" "$([[ $rc == 0 ]]; echo $?)"

  echo "Scenario: CH04 の後の木は、database を作る書き方の検査を通る"
  set +e; lint "$PWD" >/dev/null 2>&1; rc=$?; set -e
  want "本物の木で --lint が rc=0" "$([[ $rc == 0 ]]; echo $?)"

  [[ $fail == 0 ]] && echo "self-test OK" || { echo "self-test FAILED" >&2; exit 1; }
}

case "${1:-}" in
  --lint) lint "$PWD" && echo "check-test-db-growth: OK" ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 --lint | --self-test" >&2; exit 2 ;;
esac
