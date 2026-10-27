# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-10-25 10.2

- 旧: `bash -c 'd=$(git diff origin/main...HEAD -- crates/server); for k in "Scenario:" "fn "; do r=$(printf "%s\n" "$d" | grep -E "^-[^-].*$k" | sed "s/^-//" | sort); a=$(printf "%s\n" "$d" | grep -E "^\+[^+].*$k" | sed "s/^+//" | sort); [ -z "$(comm -23 <(printf "%s\n" "$r") <(printf "%s\n" "$a") | grep -v "^$")" ] || exit 1; done'`
- 新: `tools/check-tests-kept.sh origin/main crates/server`
- 理由: 旧コマンドは行の完全一致で比べ、補助関数 app_in の引数の変更を「消えた」と見て、行頭の fn も拾えない。Scenario の印は行で、fn は名前で比べる（Q5）
- 実証: evidence.jsonl の 2026-10-24T16:22:41+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261025-012241-evidence-10.2-0-f2ab11-vlj83mmoi.log、HEAD cb347fc）
- 承認: 人間: 2026-10-25 CH04 第 2 回の深掘り Q5 の答え（deep-answers-2.txt、推奨のまま）
