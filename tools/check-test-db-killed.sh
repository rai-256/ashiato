#!/usr/bin/env bash
# 試験を途中で殺しても、次の回の後に残りが無いことの検査（CH05 / design D2）。
#
#   ./tools/check-test-db-killed.sh              → 探りの試験を殺し、次の回の後に残りが無いことを見る（rc=0 通過 / 次の回の rc / rc=1 残った・空振り・印が来ない）
#   ./tools/check-test-db-killed.sh --self-test  → 偽の探り・次の回・一覧・統計で、落ちるべきものが落ち、通すべきものが通ることを確かめる
#
# 2 つの時点を順に行う（どちらかが落ちれば、その時点の名前を出して落ちる）:
#   insert    探りの試験 `ch05_killed_probe`（`crates/server/src/killed_probe_tests.rs`）が、回の database への 20,001 件の INSERT を
#             走らせている最中（`pg_sleep(600)` を前に置いた 1 文）。印の `inserting` と、`pg_stat_activity` の `ch05_probe` の印のある `active` の文を待って殺す
#   committed 回の database と試験ごとの database の両方に 20,001 件を確定した後（印の `committed`）。眠っているところを殺す
# 殺し方: `cargo test … --no-run --message-format=json` で試験の実行ファイルの path を取り、`setsid` で直接起こし（`ASHIATO_TEST_STATEMENT_TIMEOUT=0`。
#   CH04 D7 の 60 秒の上限で探りが先に落ちないように）、プロセスグループごと `kill -KILL`。
#
# 1 つの時点の流れ:
#   一覧 S0（`postgres` database から読んだ `pg_database` の名前）・`ashiato` の `core.event` の行数（表が無ければ 0）・`pg_database_size('ashiato')` を読む
#   → 探りを起こして印を待ち、殺す → 一覧 S1。殺した回の database = S1 にあって S0 に無いもの（探りが印のファイルへ書いた名前もここに入っていること）
#   → 次の回（`cargo test -p ashiato-server --lib ch05_next_run`。CH04 の回の始まりの片付けを起こす。秒数を測って出す）→ 一覧 S2・`ashiato` の行数と大きさ。
# 判定（順に。最初に外れたもので落ちる）:
#   探りの印が待ちの上限までに来ない → 殺した回の database が 1 つも見えない（検査が空振り）→ 次の回の rc（0 でなければその rc）
#   → 殺した回の database のうち S2 に残るもの（名前を挙げる）→ `ashiato` の行数が殺す前と違う → `ashiato` の大きさが殺す前から 1 MiB を超えて増えた。
# 大きさに 1 MiB の許しがあるのは、触らない database でも統計や空き領域の地図で大きさが少し動くため。行数は厳密に見る。
#
# 差し替え（`--self-test` が使う。CH04 の check-test-db-growth.sh の TEST_DB_CMD と同じ形）:
#   TEST_DB_KILLED_PROBE_CMD  探り。環境変数 CH05_PROBE_AT・CH05_PROBE_MARK を受ける（差し替えたときは pg_stat_activity を見ない）
#   TEST_DB_KILLED_NEXT_CMD   次の回
#   TEST_DB_KILLED_LIST_CMD   一覧。1 行 1 名前を標準出力へ
#   TEST_DB_KILLED_STAT_CMD   `ashiato` の「行数 大きさ（バイト）」を 1 行で標準出力へ
#   TEST_DB_KILLED_WAIT_S     印を待つ上限の秒数（既定 120）
#
# **同じ worktree で別の `cargo test` と並べて走らせない**（並んだ回の database が S1 と S0 の差に入る）。
# 検証の準備（`HARNESS_VERIFY=1 tools/verify-env.sh`）とも並べない（CH06 が DB ごと作り直す）。
#
# なぜ要るか（CH05）: 打ち切られた run の回の database と、その中の行が次の回の後にも残り、`ashiato` の隣で積もって試験の DB が膨れた。
# CH04 の `tools/check-test-db-growth.sh` は最後まで走った回だけを見るので、殺された回は別の検査で見る。
set -euo pipefail
cd "$(dirname "$0")/.."

WAIT_S=${TEST_DB_KILLED_WAIT_S:-120}
SIZE_SLACK=1048576  # 1 MiB

psql_db() {  # psql_db <database> <sql>
  tools/db.sh exec -T db psql -U "${POSTGRES_USER:-ashiato}" -d "$1" -tA -c "$2"
}
list_dbs() {
  if [[ -n ${TEST_DB_KILLED_LIST_CMD:-} ]]; then bash -c "$TEST_DB_KILLED_LIST_CMD"
  else psql_db postgres 'SELECT datname FROM pg_database ORDER BY 1'; fi | sort -u
}
# 「行数 大きさ」。表が無ければ行数は 0（CH06 の後の ashiato は空から立ち、移行は server の試験の初回まで当たらない）
stat_ashiato() {
  if [[ -n ${TEST_DB_KILLED_STAT_CMD:-} ]]; then bash -c "$TEST_DB_KILLED_STAT_CMD"; return; fi
  local rows size
  rows=$(psql_db ashiato "SELECT CASE WHEN to_regclass('core.event') IS NULL THEN 0
    ELSE (xpath('/row/c/text()', query_to_xml('SELECT count(*) AS c FROM core.event', false, true, '')))[1]::text::bigint END")
  size=$(psql_db postgres "SELECT pg_database_size('ashiato')")
  echo "$rows $size"
}
# 一覧・統計は `|| return` の中で呼ばれて set -e が効かないので、rc と中身をここで明示的に確かめる
list_checked() {  # list_checked <変数名>
  local out
  out=$(list_dbs) || { echo "database の一覧が読めない（rc=$?）" >&2; return 1; }
  [[ -n $out ]] || { echo "database の一覧が空（postgres が無いはずはない）" >&2; return 1; }
  printf -v "$1" %s "$out"
}
stat_checked() {  # stat_checked <行数の変数名> <大きさの変数名>
  local out rc=0 r s extra
  out=$(stat_ashiato) || rc=$?
  if [[ $rc != 0 ]]; then echo "ashiato の行数と大きさが読めない（rc=$rc）" >&2; return 1; fi
  read -r r s extra <<<"$out" || true
  if ! [[ $r =~ ^[0-9]+$ && $s =~ ^[0-9]+$ && -z $extra ]]; then
    echo "ashiato の行数と大きさが数値 2 つで返らなかった: '$out'" >&2; return 1
  fi
  printf -v "$1" %s "$r"; printf -v "$2" %s "$s"
}
# probe_active <回の database>: その database に ch05_probe の印のある active の文が見えれば 0、まだ見えなければ 1、
# 名前が database の名前でない・pg_stat_activity が読めなければ理由を出して 2（待ち続けて「印が来なかった」と取り違えない。review/code.md R25）
probe_active() {
  local out
  [[ $1 =~ ^[a-z0-9_]+$ ]] || { echo "探りが印に書いた回の database の名前が読めない: '$1'" >&2; return 2; }
  out=$(psql_db postgres "SELECT 1 FROM pg_stat_activity WHERE state = 'active' AND datname = '$1' AND query LIKE '%ch05_probe%' AND pid <> pg_backend_pid() LIMIT 1") \
    || { echo "pg_stat_activity が読めない（psql の rc=$?）" >&2; return 2; }
  [[ -n $out ]]
}
run_next() {  # 次の回。出力は標準出力へ。rc を返す
  if [[ -n ${TEST_DB_KILLED_NEXT_CMD:-} ]]; then bash -c "$TEST_DB_KILLED_NEXT_CMD"; return; fi
  local out rc=0
  out=$(cargo test -p ashiato-server --lib ch05_next_run 2>&1) || rc=$?
  echo "$out"
  [[ $rc != 0 ]] && return "$rc"
  grep -q ' 1 passed' <<<"$out" || { echo "ch05_next_run が 1 本も走らなかった（絞り込みの名前が試験と合っていない）" >&2; return 1; }
}

exe_path() {  # 試験の実行ファイルの path。作れなければ cargo の標準エラーとコンパイルのエラーを出して rc≠0
  local json rc=0
  json=$(cargo test -p ashiato-server --lib --no-run --message-format=json 2>"$dir/cargo.err") || rc=$?
  if [[ $rc != 0 ]]; then
    echo "試験の実行ファイルを作れない（cargo の rc=$rc）:" >&2
    jq -r 'select(.reason == "compiler-message" and .message.level == "error") | .message.rendered' <<<"$json" >&2 || true
    tail -n 40 "$dir/cargo.err" >&2
    return 1
  fi
  jq -r 'select(.reason == "compiler-artifact" and .profile.test == true and .executable != null) | .executable' <<<"$json" | tail -1
}

# timed_next <見出し>: 次の回を走らせ、「<見出し>: rc=<n> <秒> 秒」を出す。落ちればその出力を出し、その rc を返す
timed_next() {
  local t0=$EPOCHREALTIME t1 out rc=0
  out=$(run_next 2>&1) || rc=$?
  t1=$EPOCHREALTIME
  echo "$1: rc=$rc $(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.1f", b-a}') 秒"
  if [[ $rc != 0 ]]; then echo "$out" >&2; echo "$1の試験が rc=$rc で落ちた" >&2; return "$rc"; fi
}

probe_pid=""
kill_probe() {
  if [[ -n $probe_pid ]]; then
    kill -KILL -- "-$probe_pid" 2>/dev/null || true
    wait "$probe_pid" 2>/dev/null || true
    probe_pid=""
  fi
}

# start_probe <時点> <印のファイル> <試験の実行ファイル>
start_probe() {
  local at=$1 mark=$2 exe=$3
  : >"$mark"
  if [[ -n ${TEST_DB_KILLED_PROBE_CMD:-} ]]; then
    CH05_PROBE_AT=$at CH05_PROBE_MARK=$mark setsid bash -c "$TEST_DB_KILLED_PROBE_CMD" >"$mark.log" 2>&1 &
  else
    (cd crates/server && ASHIATO_TEST_STATEMENT_TIMEOUT=0 CH05_PROBE_AT=$at CH05_PROBE_MARK=$mark \
      exec setsid "$exe" --ignored --exact killed_probe_tests::ch05_killed_probe --nocapture) >"$mark.log" 2>&1 &
  fi
  probe_pid=$!
}

# wait_mark <時点> <印のファイル> : 印（と insert なら走っている文）を待つ。上限を越えたら rc=1
wait_mark() {
  local at=$1 mark=$2 want deadline run arc
  want=inserting; [[ $at == committed ]] && want=committed
  deadline=$(( SECONDS + WAIT_S ))
  while (( SECONDS < deadline )); do
    if grep -qx "$want" "$mark" 2>/dev/null; then
      if [[ $at == insert && -z ${TEST_DB_KILLED_PROBE_CMD:-} ]]; then
        run=$(sed -n 's/^run=//p' "$mark" | head -1)
        arc=0; probe_active "$run" || arc=$?
        case $arc in
          0) return 0 ;;
          2) echo "時点 $at: 探りの文が走っているかを確かめられない" >&2; return 1 ;;
        esac
      else return 0; fi
    fi
    if ! kill -0 "$probe_pid" 2>/dev/null; then
      echo "時点 $at: 探りの印が来る前に探りが終わった（探りが走らなかった・0 本の絞り込み）" >&2
      tail -n 20 "$mark.log" >&2 || true; return 1
    fi
    sleep 0.2
  done
  echo "時点 $at: 探りの印（$want）が ${WAIT_S} 秒の待ちの上限までに来なかった（探りが走らなかった・0 本の絞り込み）" >&2
  tail -n 20 "$mark.log" >&2 || true
  return 1
}

# point <時点> <作業の場所> <試験の実行ファイル>
point() {
  local at=$1 dir=$2 exe=$3 mark="$2/mark" s0 s1 s2 killed left name rows0 size0 rows1 size1 rc
  list_checked s0 || return 1
  stat_checked rows0 size0 || return 1
  start_probe "$at" "$mark" "$exe"
  wait_mark "$at" "$mark" || { kill_probe; return 1; }
  kill_probe
  list_checked s1 || return 1
  killed=$(comm -13 <(echo "$s0") <(echo "$s1"))
  if [[ -z $killed ]]; then
    echo "時点 $at: 殺した回の database が見えない（検査が空振り）" >&2; return 1
  fi
  for name in $(sed -n 's/^\(run\|iso\)=//p' "$mark"); do
    grep -qx "$name" <<<"$killed" || { echo "時点 $at: 探りが印に書いた database $name が、殺した後の一覧に増えていない" >&2; return 1; }
  done
  rc=0; timed_next "時点 $at: 次の回" || rc=$?
  [[ $rc == 0 ]] || return "$rc"
  list_checked s2 || return 1
  left=$(comm -12 <(echo "$killed") <(echo "$s2"))
  if [[ -n $left ]]; then
    echo "時点 $at: 殺した回の database が次の回の後にも残っている:" >&2; sed 's/^/  /' <<<"$left" >&2
    echo "  → 次の回の始まりの片付け（testdb.rs）が消していない" >&2; return 1
  fi
  stat_checked rows1 size1 || return 1
  if [[ $rows1 != "$rows0" ]]; then
    echo "時点 $at: ashiato database の core.event の行数が変わった（前 $rows0 → 後 $rows1）" >&2; return 1
  fi
  if (( size1 > size0 + SIZE_SLACK )); then
    echo "時点 $at: ashiato database の大きさが 1 MiB を超えて増えた（前 $size0 → 後 $size1 バイト）" >&2; return 1
  fi
  echo "時点 $at: OK"
  case $at in
    insert) echo "Scenario: 体積の行を入れる途中で殺した回の database は、次の回の後に残らない" ;;
    committed) echo "Scenario: 体積の行を確定した後に殺した回の database は、次の回の後に残らない" ;;
  esac
}

killed_check() {
  local exe="" at
  dir=$(mktemp -d)  # trap が関数の外で走るので大域
  trap 'kill_probe; rm -rf "$dir"' EXIT
  if [[ -z ${TEST_DB_KILLED_PROBE_CMD:-} ]]; then
    exe=$(exe_path) || return 1
    [[ -n $exe && -x $exe ]] || { echo "試験の実行ファイルの path が取れなかった" >&2; return 1; }
  fi
  if [[ -z ${TEST_DB_KILLED_NEXT_CMD:-} ]]; then  # 殺す前のきれいな回の秒数（出す）。きれいな回が落ちれば、殺す前にその rc で落ちる
    local brc=0
    timed_next "殺す前のきれいな回" || brc=$?
    [[ $brc == 0 ]] || return "$brc"
  fi
  for at in insert committed; do
    point "$at" "$dir" "$exe" || return $?
  done
  echo "check-test-db-killed: OK（殺した回の分は次の回の後に残らず、ashiato は増えていない）"
}

self_test() {
  local fail=0 out rc d
  tmp=$(mktemp -d)  # trap が関数の外で走るので大域
  trap 'rm -rf "$tmp"' EXIT
  want() {  # want <説明> <条件の rc>
    if [[ $2 == 0 ]]; then echo "  self-test ok: $1"; else echo "  self-test NG: $1" >&2; fail=1; fi
  }
  # k <次の回の命令> [探りの命令] [待ちの上限] : 偽の探り・次の回・一覧・統計で killed_check を走らせる。
  # 偽の database 一覧は $d/dbs、偽の統計は $d/stat（次の回の命令が書き換える）
  k() {
    d="$tmp/k"; rm -rf "$d"; mkdir -p "$d"
    printf 'ashiato\npostgres\n' >"$d/dbs"; echo '10 5000000' >"$d/stat"
    local next=$1 probe=${2:-} wait=${3:-5} statcmd=${4:-cat $d/stat}
    # 既定の探り: 回と試験ごとの database を作り、印のファイルへ名前と時点の印を書いて眠る
    [[ -z $probe ]] && probe="r=ashiato_run_aaaaaaaaaaaa; echo \$r >>$d/dbs; echo \${r}_x_111111111111 >>$d/dbs
      echo run=\$r >>\$CH05_PROBE_MARK; echo iso=\${r}_x_111111111111 >>\$CH05_PROBE_MARK
      if [ \$CH05_PROBE_AT = insert ]; then echo inserting >>\$CH05_PROBE_MARK; else echo committed >>\$CH05_PROBE_MARK; fi; sleep 600"
    set +e
    out=$(TEST_DB_KILLED_PROBE_CMD="$probe" TEST_DB_KILLED_NEXT_CMD="$next" TEST_DB_KILLED_LIST_CMD="cat $d/dbs" \
      TEST_DB_KILLED_STAT_CMD="$statcmd" TEST_DB_KILLED_WAIT_S="$wait" bash "$0" 2>&1)
    rc=$?; set -e
  }
  sweep="sed -i '/^ashiato_run_/d' $tmp/k/dbs"

  echo "Scenario: 殺した回の database が次の回の後に残っていれば、名前を挙げて落ちる"
  k "true"
  want "残れば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "残った database の名前を挙げる" "$(grep -q 'ashiato_run_aaaaaaaaaaaa' <<<"$out"; echo $?)"
  want "時点の名前を挙げる" "$(grep -q '時点 insert' <<<"$out"; echo $?)"

  echo "Scenario: ashiato database の行数が増えていれば落ちる"
  k "$sweep; echo '11 5000000' >$tmp/k/stat"
  want "行数が増えれば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "前と後の行数を挙げる" "$(grep -q '前 10 → 後 11' <<<"$out"; echo $?)"

  echo "Scenario: ashiato database の大きさだけが 1 MiB を超えて増えていれば落ちる"
  k "$sweep; echo '10 6048577' >$tmp/k/stat"
  want "1 MiB と 1 バイト増えれば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "前と後の大きさを挙げる" "$(grep -q '前 5000000 → 後 6048577' <<<"$out"; echo $?)"
  k "$sweep; echo '10 6048576' >$tmp/k/stat"
  want "ちょうど 1 MiB の増えは許す" "$([[ $rc == 0 ]]; echo $?)"

  echo "Scenario: 殺した回の database が見えなければ、空振りとして落ちる"
  k "$sweep" "echo inserting >>\$CH05_PROBE_MARK; sleep 600"
  want "database が増えなければ rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "見えないことを挙げる" "$(grep -q '殺した回の database が見えない' <<<"$out"; echo $?)"

  echo "Scenario: 探りの印が来なければ、待ちの上限で時点の名前を挙げて落ちる"
  k "$sweep" "sleep 600" 1
  want "印が来なければ rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "印が来なかったことを挙げる" "$(grep -q '印.*来なかった' <<<"$out"; echo $?)"
  want "時点の名前を挙げる" "$(grep -q '時点 insert' <<<"$out"; echo $?)"

  echo "Scenario: 次の回の試験が落ちれば、台本も試験の終了コードで落ちる"
  k "exit 101"
  want "次の回が rc=101 なら台本も rc=101" "$([[ $rc == 101 ]]; echo $?)"

  echo "Scenario: 殺した回の分が片付き、ashiato が増えていなければ、台本は通る"
  k "$sweep"
  want "片付けば rc=0" "$([[ $rc == 0 ]]; echo $?)"
  want "2 つの時点を通る" "$(grep -q '時点 insert: OK' <<<"$out" && grep -q '時点 committed: OK' <<<"$out"; echo $?)"

  echo "-- 統計や一覧が読めない・数値でなければ、黙って通らず落ちる"
  k "true" "" 5 "exit 3"
  want "統計が rc=3 なら rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "読めないことを挙げる" "$(grep -q '読めない' <<<"$out"; echo $?)"
  k "true" "" 5 "echo abc"
  want "数値でない統計なら rc=1" "$([[ $rc == 1 ]]; echo $?)"
  k "$sweep; echo 'x 5000000' >$tmp/k/stat"
  want "殺した後に統計が数値でなくなれば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  k "$sweep; : >$tmp/k/stat"
  want "殺した後に統計が空になれば rc=1" "$([[ $rc == 1 ]]; echo $?)"
  d="$tmp/k"; set +e
  out=$(TEST_DB_KILLED_LIST_CMD="exit 4" TEST_DB_KILLED_PROBE_CMD="sleep 600" TEST_DB_KILLED_NEXT_CMD=true \
    TEST_DB_KILLED_STAT_CMD="echo 10 5000000" TEST_DB_KILLED_WAIT_S=1 bash "$0" 2>&1); rc=$?; set -e
  want "一覧が読めなければ rc=1" "$([[ $rc == 1 ]]; echo $?)"
  want "一覧が読めないことを挙げる" "$(grep -q '一覧が読めない' <<<"$out"; echo $?)"

  echo "Scenario: CI の rust job は、殺した後の検査を走らせる"
  # rust job の `- run:` 行だけを見る（コメントや別の job の行を数えない）
  local runs
  runs=$(awk '/^  rust:/{f=1; next} /^  [A-Za-z0-9_-]+:/{f=0} f && /^ *- run:/' .github/workflows/ci.yml)
  want "ci.yml の rust job に --self-test の - run: 行がある" "$(grep -qxE ' *- run: \./tools/check-test-db-killed\.sh --self-test *' <<<"$runs"; echo $?)"
  want "ci.yml の rust job に本体の - run: 行がある" "$(grep -qxE ' *- run: \./tools/check-test-db-killed\.sh *' <<<"$runs"; echo $?)"

  [[ $fail == 0 ]] && echo "self-test OK" || { echo "self-test FAILED" >&2; exit 1; }
}

case "${1:-}" in
  --self-test) self_test ;;
  -h|--help) echo "使い方: $0 | --self-test" ;;
  "") killed_check ;;
  *) echo "使い方: $0 | --self-test" >&2; exit 2 ;;
esac
