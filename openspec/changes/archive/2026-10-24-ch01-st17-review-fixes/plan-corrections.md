# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-10-22 5.3

- 旧: `python3 scripts/story_brief.py ST23 | grep -q 'ch01-st17-review-fixes'`
- 新: `tools/check-handoff-reaches-brief.sh ST23 ch01-st17-review-fixes`
- 理由: story_brief.py は HTML のパスを標準出力に出すだけで本文を出さないため、パイプの grep は原理的に通らない。書かれた HTML を読む道具に替える
- 実証: evidence.jsonl の 2026-10-22T11:45:56+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261022-204556-evidence-5.3-0-bed6fc.log、HEAD 1dbdb55）
- 承認: machine: evidence.jsonl 2026-10-22T11:45:58+00:00 PASS
