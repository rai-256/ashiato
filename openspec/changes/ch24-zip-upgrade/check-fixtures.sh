#!/usr/bin/env bash
# CH24 1.5: 埋め込みの囲み（CH24-FIXTURE-BEGIN〜END）が zip 2.6.1 の木で書かれ、上げた後に書き直していないことを確かめる。
#
# 3 つのファイルそれぞれで:
#   1. 囲みを最初に入れた commit の Cargo.lock の zip が 2.6.1 である
#   2. そのファイルを触った commit のうち、Cargo.lock の zip が 2.6.1 だった最も新しい commit（= 上げる前の最後の版）の
#      囲みの中が、今の作業ツリーの囲みの中と同じである
# 旧コマンドは 1 の commit（最初の版）と比べていたため、zip を上げる前の Task 1 の中での正当な直し（レビューの指摘）も
# 「作り直し」として落としていた。2 にすることで、上げた後（Cargo.lock の zip が 2.6.1 でない commit・未 commit の変更）の
# 書き直しだけを捕まえる。
set -euo pipefail

zip_version() { # <commit> → Cargo.lock の zip の版（複数なら空白区切り）
  git show "$1:Cargo.lock" | python3 -c 'import sys,re;print(" ".join(re.findall(r"name = \"zip\"\nversion = \"([^\"]+)\"", sys.stdin.read())))'
}
block() { sed -n '/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p'; }

for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do
  grep -q CH24-FIXTURE-BEGIN "$f" && grep -q CH24-FIXTURE-END "$f" || { echo "囲みが無い（作業ツリー）: $f"; exit 1; }

  first=$(git log --format=%H -S CH24-FIXTURE-BEGIN HEAD -- "$f" | tail -1)
  test -n "$first" || { echo "囲みを入れた commit が無い: $f"; exit 1; }
  v=$(zip_version "$first")
  echo "$f: 囲みを入れた commit ${first:0:9} の zip = $v"
  test "$v" = "2.6.1" || { echo "囲みを入れた commit の zip が 2.6.1 でない: $f"; exit 1; }

  last=""
  for c in $(git log --format=%H HEAD -- "$f"); do
    git grep -q CH24-FIXTURE-BEGIN "$c" -- "$f" || continue
    if [ "$(zip_version "$c")" = "2.6.1" ]; then last=$c; break; fi
  done
  test -n "$last" || { echo "zip 2.6.1 の木で囲みを持つ commit が無い: $f"; exit 1; }
  echo "$f: 上げる前の最後の版 ${last:0:9} と比べる"
  diff <(git show "$last:$f" | block) <(block < "$f") || { echo "囲みの中が変わった: $f"; exit 1; }
done
echo "OK: 3 つのファイルの囲みは zip 2.6.1 の木で書いた版のまま"
