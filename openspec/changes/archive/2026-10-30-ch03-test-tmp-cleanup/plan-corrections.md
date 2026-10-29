# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-10-25 8.3

- 旧: `bash -c 'git fetch -q origin main; d=$(git diff -U0 origin/main...HEAD -- crates collector-android/app/src); pick(){ printf "%s\n" "$d" | grep -E "^$1" | grep -oE "Scenario: .*|fn [A-Za-z0-9_]+|fun .[^(]*" | grep -vxE "fn (temp_dir|new|drop|path)" | sort -u; }; [ -z "$(comm -23 <(pick -) <(pick "\\+"))" ] && b=$(git grep -hcE "#\[(tokio::)?test" origin/main -- crates | paste -sd+ | bc) && h=$(git grep -hcE "#\[(tokio::)?test" HEAD -- crates | paste -sd+ | bc) && [ "$h" -ge "$b" ] && kb=$(git grep -hc "@Test" origin/main -- collector-android/app/src/test | paste -sd+ | bc) && kh=$(git grep -hc "@Test" HEAD -- collector-android/app/src/test | paste -sd+ | bc) && [ "$kh" -ge "$kb" ]'`
- 新: `openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh`
- 理由: 試験の本数を先端の origin/main と比べていて、ブランチの後に他の Story が main へ足した試験の分だけ必ず落ちる（先端 1694/626・分岐点 1507/474・HEAD 1508/476）。受け入れ条件（分岐点より減っていない）は変えず、比べる相手を merge-base にした
- 実証: evidence.jsonl の 2026-10-25T09:56:59+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261025-185659-evidence-8.3-0-a7b58e-vj7by8yoy.log、HEAD dd23266）
- 承認: machine: evidence.jsonl 2026-10-25T09:57:27+00:00 PASS
