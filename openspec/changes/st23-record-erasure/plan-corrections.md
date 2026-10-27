# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-10-24 1.1

- 旧: `tools/check-immutable.d/<時刻>_st23-derived-erasure.sh`
- 新: `tools/check-st23-derived-erasure.sh`
- 理由: <時刻> は名前の雛形で、そのまま走らせると bash の構文エラーになる。節は check-immutable.sh が . で読む形（psql・fail を共有）で単独では走らないため、check-immutable.sh を走らせ節の見出し・NG なし・節の OK 行 10 本以上を確かめる入口に直す
- 実証: evidence.jsonl の 2026-10-23T23:45:14+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261024-084514-evidence-1.1-2-dae058.log、HEAD b5cf278）
- 承認: machine: evidence.jsonl 2026-10-23T23:42:23+00:00 PASS
