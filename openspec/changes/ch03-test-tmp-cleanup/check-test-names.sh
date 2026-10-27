#!/usr/bin/env bash
# CH03 の Task 8.3（design D7）: 試験の印と名前が消えておらず、試験の本数が減っていない。
# 本数は分岐点（merge-base）と比べる。先端の origin/main と比べると、ブランチの後に他の Story が
# 足した試験の分だけ必ず落ちる（2026-10-25 実測: 先端 1694/626、分岐点 1507/474、HEAD 1508/476）。
set -euo pipefail
git fetch -q origin main
m=$(git merge-base origin/main HEAD)
d=$(git diff -U0 origin/main...HEAD -- crates collector-android/app/src)
pick() { printf "%s\n" "$d" | grep -E "^$1" | grep -oE "Scenario: .*|fn [A-Za-z0-9_]+|fun .[^(]*" | grep -vxE "fn (temp_dir|new|drop|path)" | sort -u || true; }
gone=$(comm -23 <(pick -) <(pick "\\+"))
[ -z "$gone" ] || { echo "消えた印・名前:"; echo "$gone"; exit 1; }
count() { git grep -hcE "$1" "$2" -- "$3" | paste -sd+ | bc; }
b=$(count "#\[(tokio::)?test" "$m" crates); h=$(count "#\[(tokio::)?test" HEAD crates)
kb=$(count "@Test" "$m" collector-android/app/src/test); kh=$(count "@Test" HEAD collector-android/app/src/test)
echo "rust ${b} -> ${h} / kotlin ${kb} -> ${kh}（分岐点 ${m:0:8} → HEAD）"
[ "$h" -ge "$b" ] && [ "$kh" -ge "$kb" ] || { echo "試験の注釈の数が分岐点より減った"; exit 1; }
# 本数の総数だけだと、CH03 が足した分（rust 1・kotlin 2）までは既存の試験の注釈を外しても通る（R20）。
# 注釈の直後の関数の名前の集合で、分岐点の試験が全部 HEAD でも試験であることを見る
tests() { # rev 注釈の正規表現 置き場 関数の正規表現
  git grep -h -A6 -E "$2" "$1" -- "$3" | A=$2 F=$4 awk '
    $0 ~ ENVIRON["A"] { want = 1; next }
    want && match($0, ENVIRON["F"]) { print substr($0, RSTART, RLENGTH); want = 0 }' | sort -u
}
rs='^[[:space:]]*#\[(tokio::)?test' rf='fn [A-Za-z0-9_]+'
ks='^[[:space:]]*@Test' kf='fun (`[^`]+`|[A-Za-z0-9_]+)'
[ "$(tests "$m" "$rs" crates "$rf" | wc -l)" -gt 0 ] && [ "$(tests "$m" "$ks" collector-android/app/src/test "$kf" | wc -l)" -gt 0 ] \
  || { echo "分岐点の試験の名前を 1 つも拾えない（拾い方が壊れている）"; exit 1; }
lost=$( { comm -23 <(tests "$m" "$rs" crates "$rf") <(tests HEAD "$rs" crates "$rf")
          comm -23 <(tests "$m" "$ks" collector-android/app/src/test "$kf") <(tests HEAD "$ks" collector-android/app/src/test "$kf"); } )
[ -z "$lost" ] || { echo "試験でなくなった関数（分岐点では試験の注釈があった）:"; echo "$lost"; exit 1; }
