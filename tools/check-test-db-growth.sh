#!/usr/bin/env bash
# 試験が database を作る書き方の検査（CH04 / design D5）。
#
#   ./tools/check-test-db-growth.sh [cargo test の引数…]  → 試験を 2 回続けて走らせ、database が積もらないことを見る
#                                                            （引数が無ければ `-p ashiato-server`。rc=0 通過 / 試験の rc / rc=1 積もった）
#                                                            `--lint`・`--self-test`・`-h`・`--help` 以外の引数（`--workspace` なども）はそのまま cargo test へ渡す
#   ./tools/check-test-db-growth.sh --lint       → 本物の木を検査する（rc=0 通過 / rc=1 違反 / rc=2 使い方）
#   ./tools/check-test-db-growth.sh --self-test  → 偽の木で、落ちるべきものが落ち、通すべきものが通ることを確かめる
#
# 見るのは 2 つ（どちらも `crates/server` の下）:
#   1. `src` と `tests` の `CREATE DATABASE` —— `testdb.rs` の中だけに書く。ほかは `testdb::isolated()`（回のテンプレートの写し）か
#      `testdb::scratch()`（空）を使う。
#   2. `src/export*_tests.rs` の `testdb::pool()` —— 書き出しの試験は回の database を共有せず、試験ごとの database（`testdb::isolated()`）を取る。
# 通すのは `testdb.rs` と、同じ行か 1 行上に理由つきの `db-ok: <理由>` がある行。
#
# 積もりの検査（design D8）: 一覧 S0 → 試験 1 回目 → 一覧 S1 → 試験 2 回目 → 一覧 S2（一覧は postgres database から読んだ `pg_database` の名前）。
# 判定は順に、最初に外れたもので落ちる:
#   試験の rc（0 でなければその rc）→ 前の回の残り（S1 にあって S0 に無く、S2 にも残る）→ 最後の回の余計な残り（S2 にあって S1 に無い名前のうち、
#   `_x_`・`_tmp_` を含むもの、または `ashiato_run_` の回の id が 2 つ以上）→ S0 にあった `ashiato` が S2 に無い。落ちるときは名前を挙げる。
# 差し替え: `TEST_DB_CMD`（試験の命令。既定は `cargo test <引数>`）・`TEST_DB_LIST_CMD`（一覧。1 行 1 名前を標準出力へ）。
# **同じ worktree で別の `cargo test` と並べて走らせない**（並んだ回の database が S1 と S2 の差に入る）。
#
# なぜ要るか（CH04）: 試験の database の作り方が 5 か所に散り、回の印を持たない `st*_tmp_…` が `ashiato` の隣に溜まって
# 試験が遅くなった。走っている Story が新しく足す所も、この検査が rebase の後に止める。
set -euo pipefail
cd "$(dirname "$0")/.."

# lint <木の根> : 違反を「ファイル:行: 内容」で出し、あれば rc=1
lint() {
  local root=$1 bad=0 hits
  # 1 つ目: CREATE DATABASE。`file:line:text` を挙げ、同じ行か 1 行上の db-ok: で通す
  hits=$(cd "$root" && grep -rniE 'CREATE DATABASE' crates/server/src crates/server/tests --include='*.rs' 2>/dev/null \
    | grep -v '^crates/server/src/testdb\.rs:' || true)
  # 行頭が // の説明の行（/// も）は、書き方でなく文章なので見ない
  hits=$(grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' <<<"$hits" || true)
  report "$root" "$hits" "CREATE DATABASE は testdb.rs の中だけに書く。試験ごとの写しは testdb::isolated()、空の使い捨ては testdb::scratch() を使う。持ち主のいない回の database を作る片付けの試験などは、同じ行か 1 行上に db-ok: <理由> を書く" || bad=1
  # 2 つ目: 書き出しの試験の testdb::pool()。Rust の識別子は大文字と小文字を分けるので、わざと字のとおりに比べる
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

# ---- 積もりの検査 ----
list_dbs() {
  if [[ -n ${TEST_DB_LIST_CMD:-} ]]; then bash -c "$TEST_DB_LIST_CMD"
  else tools/db.sh exec -T db psql -U "${POSTGRES_USER:-ashiato}" -d postgres -tA -c 'SELECT datname FROM pg_database ORDER BY 1'; fi
}
run_tests() {
  if [[ -n ${TEST_DB_CMD:-} ]]; then bash -c "$TEST_DB_CMD"; else cargo test "$@"; fi
}

# growth [cargo test の引数…]
growth() {
  local s0 s1 s2 rc left extra ids
  [[ $# == 0 ]] && set -- -p ashiato-server
  if [[ -z ${TEST_DB_CMD:-} ]]; then cargo test --no-run "$@"; fi
  s0=$(list_dbs | sort -u)
  rc=0; run_tests "$@" || rc=$?
  [[ $rc != 0 ]] && { echo "試験 1 回目が rc=$rc で落ちた" >&2; return "$rc"; }
  s1=$(list_dbs | sort -u)
  rc=0; run_tests "$@" || rc=$?
  [[ $rc != 0 ]] && { echo "試験 2 回目が rc=$rc で落ちた" >&2; return "$rc"; }
  s2=$(list_dbs | sort -u)

  # 前の回の残り: S1 にあって S0 に無い名前のうち S2 に残るもの
  left=$(comm -12 <(comm -13 <(echo "$s0") <(echo "$s1")) <(echo "$s2"))
  if [[ -n $left ]]; then
    echo "前の回の database が 2 回目の後にも残っている:" >&2; sed 's/^/  /' <<<"$left" >&2
    echo "  → 次の回の始まりの片付け（testdb.rs）が消していない" >&2; return 1
  fi
  # 最後の回の余計な残り: S2 にあって S1 に無い名前のうち、試験ごと・使い捨て、または回の id が 2 つ以上
  extra=$(comm -13 <(echo "$s1") <(echo "$s2"))
  left=$(grep -E '_x_|_tmp_' <<<"$extra" || true)
  if [[ -n $left ]]; then
    echo "2 回目の試験ごと・使い捨ての database が残っている:" >&2; sed 's/^/  /' <<<"$left" >&2
    echo "  → TestDb の Drop が消していない" >&2; return 1
  fi
  ids=$(grep -oE '^ashiato_run_[0-9a-f]{12}' <<<"$extra" | sort -u || true)
  if [[ -n $ids && $(wc -l <<<"$ids") -ge 2 ]]; then
    echo "最後の回の id が 2 つ以上ある:" >&2; sed 's/^/  /' <<<"$ids" >&2; return 1
  fi
  if grep -qx ashiato <<<"$s0" && ! grep -qx ashiato <<<"$s2"; then
    echo "ashiato database が消えている" >&2; return 1
  fi
  echo "check-test-db-growth: OK（残りは最後の回の分だけ）"
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
  printf 'sqlx::raw_sql("create database x");\n' >"$tmp/t/crates/server/src/x_tests.rs"
  run
  want "小文字の create database も rc=1" "$([[ $rc == 1 ]]; echo $?)"

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

  # ---- 積もりの検査 ----
  # g <S0> <S1> <S2> [試験の命令] : 偽の一覧 3 つで growth を走らせる（一覧は 1 行 1 名前をスペース区切りで渡す）
  g() {
    local d="$tmp/g"; rm -rf "$d"; mkdir -p "$d"
    printf '%s\n' $1 >"$d/s0"; printf '%s\n' $2 >"$d/s1"; printf '%s\n' $3 >"$d/s2"; echo 0 >"$d/n"
    set +e
    out=$(TEST_DB_CMD="${4:-true}" TEST_DB_LIST_CMD="n=\$(cat $d/n); echo \$((n+1)) >$d/n; cat $d/s\$n" growth 2>&1)
    rc=$?; set -e
  }
  A="ashiato postgres"
  R1="ashiato_run_aaaaaaaaaaaa"; R2="ashiato_run_bbbbbbbbbbbb"

  echo "Scenario: 前の回の database が 2 回目の後に残っていれば、名前を挙げて落ちる"
  g "$A" "$A $R1" "$A $R1 $R2"
  want "前の回が残れば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "残った名前を挙げる" "$(grep -q "$R1" <<<"$out"; echo $?)"

  echo "Scenario: 2 回目の試験ごとの database が残っていれば、名前を挙げて落ちる"
  g "$A" "$A $R1" "$A $R2 ${R2}_x_cccccccccccc"
  want "試験ごとが残れば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "残った名前を挙げる" "$(grep -q "${R2}_x_cccccccccccc" <<<"$out"; echo $?)"
  g "$A" "$A $R1" "$A $R2 ${R2}_tmp_cccccccccccc"
  want "使い捨てが残っても rc=1" "$([[ $rc == 1 ]]; echo $?)"

  echo "最後の回の id が 2 つ以上残っていれば、名前を挙げて落ちる（回の id は ashiato_run_ と 16 進 12 桁の小文字）"
  R3="ashiato_run_cccccccccccc"
  g "$A" "$A $R1" "$A $R2 $R3"
  want "回の id が 2 つなら rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "2 つの id を挙げる" "$(grep -q "$R2" <<<"$out" && grep -q "$R3" <<<"$out"; echo $?)"
  g "$A" "$A $R1" "$A $R2 ${R2}_tpl"
  want "同じ回の id の別の database は 1 つと数え rc=0" "$([[ $rc == 0 ]]; echo $?)"
  g "$A" "$A $R1" "$A $R2 ashiato_run_gggggggggggg ashiato_run_ABCDEF012345 ashiato_run_aaaaaaaaaaa"
  want "16 進 12 桁の小文字でない名前は回の id と数えず rc=0" "$([[ $rc == 0 ]]; echo $?)"

  echo "Scenario: ashiato database が消えていれば落ちる"
  g "$A" "$A $R1" "postgres $R2"
  want "ashiato が無ければ rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "ashiato を挙げる" "$(grep -q 'ashiato database' <<<"$out"; echo $?)"

  echo "Scenario: 試験が落ちれば、台本も試験の終了コードで落ちる"
  g "$A" "$A $R1" "$A $R2" "exit 101"
  want "試験が rc=101 なら台本も rc=101" "$([[ $rc == 101 ]]; echo $?)"

  echo "Scenario: 前の回が片付き、最後の回の分だけが残れば、台本は通る"
  g "$A" "$A $R1" "$A $R2"
  want "最後の回の分だけなら rc=0" "$([[ $rc == 0 ]]; echo $?)"

  echo "Scenario: CI の rust job は台本を走らせる"
  local ci=.github/workflows/ci.yml job
  job=$(awk '/^  rust:/{f=1;next} f&&/^  [a-z]/{f=0} f' "$ci")
  for line in './tools/check-test-db-growth.sh --self-test' './tools/check-test-db-growth.sh --lint' './tools/check-test-db-growth.sh -p ashiato-server ch04_'; do
    want "rust job に「$line」がある" "$(grep -qxF "      - run: $line" <<<"$job"; echo $?)"
  done

  [[ $fail == 0 ]] && echo "self-test OK" || { echo "self-test FAILED" >&2; exit 1; }
}

case "${1:-}" in
  --lint) lint "$PWD" && echo "check-test-db-growth: OK" ;;
  --self-test) self_test ;;
  -h|--help) echo "使い方: $0 [cargo test の引数…] | --lint | --self-test（ほかの引数は cargo test へそのまま渡す）" ;;
  *) growth "$@" ;;
esac
