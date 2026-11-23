# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-23 8.1

- 旧: `bash -c '! grep -nF "release/" tools/release/version.sh'`
- 新: `tools/release/check-no-release-branch.sh`
- 理由: version.sh は tools/release/ のパスを 48 か所で正当に持つので grep -F release/ は原理的に通らない。消すのは修正の枝の分岐（release/<数字>・origin 上の release/）で、それだけを見る台本に置き換える
- 実証: evidence.jsonl の 2026-11-23T03:37:24+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261123-123724-evidence-8.1-1-e89f85-vdx2a00fg.log、HEAD c8ef1f3）
- 承認: machine: evidence.jsonl 2026-11-23T03:37:33+00:00 PASS
