# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-10 6.1

- 旧: `bash -c 'python3 -c "import yaml,sys; j=yaml.safe_load(open(\".github/workflows/ci.yml\",encoding=\"utf-8\"))[\"jobs\"][\"package-windows\"][\"steps\"]; r=[s.get(\"run\",\"\") for s in j]; i=[n for n,x in enumerate(r) if \"cargo test -p ashiato-setup ch14_\" in x and \"-ge 11\" in x]; k=[n for n,x in enumerate(r) if \"package.sh windows\" in x]; sys.exit(0 if i and k and i[0]<k[0] else 1)"'`
- 新: `tools/check-ci-ch14-step.sh`
- 理由: 本文のコマンドは字面に cargo test ... ch14_ を含み、verify-run が「試験が 1 本も走っていない」と読んで rc=0 でも FAIL にする。同じ確かめ（YAML の step の有無と順序）を台本に置く
- 実証: evidence.jsonl の 2026-11-10T09:54:01+00:00（FAIL、rc=0、ログ .superpowers/sdd/logs/20261110-185401-evidence-6.1-0-75baa2-vugvn_e50.log、HEAD ee2d881）
- 承認: machine: evidence.jsonl 2026-11-10T09:54:06+00:00 PASS
