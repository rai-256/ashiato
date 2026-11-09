# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-04 1.1

- 旧: `bash -c 'diff <(git show origin/main:CONTRIBUTING.md | sed -n "/^## /,\$p") <(sed -n "/^## /,\$p" CONTRIBUTING.md)'`
- 新: `tools/check-cla-section.sh`
- 理由: 最初の見出しから後ろには 10.3 が直す「## 送る前に」の手元の検査の行（38 行目）が入り、守るべき CLA の節が変わっていなくても diff が rc=1 になる（review/code.md R14）。範囲を本文が言う CLA の節に狭める
- 実証: evidence.jsonl の 2026-11-03T17:29:26+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261104-022926-evidence-1.1-3-c8baa1-vmuo15whd.log、HEAD 720285b）
- 承認: machine: evidence.jsonl 2026-11-03T17:29:32+00:00 PASS
