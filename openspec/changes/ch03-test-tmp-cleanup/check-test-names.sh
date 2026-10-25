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
[ "$h" -ge "$b" ] && [ "$kh" -ge "$kb" ]
