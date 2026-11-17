# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-17 2.3

- 旧: `python3 -c 'import subprocess as sp, yaml; base=sp.run(["git","merge-base","HEAD","origin/main"],capture_output=True,text=True,check=True).stdout.strip(); o=yaml.safe_load(sp.run(["git","show",base+":.github/workflows/ci.yml"],capture_output=True,text=True,check=True).stdout); n=yaml.safe_load(open(".github/workflows/ci.yml",encoding="utf-8")); oj=o.pop("jobs"); nj=n.pop("jobs"); assert o==n and oj.keys()==nj.keys(); assert all(oj[k]==nj[k] for k in oj if k!="package-windows"); a=dict(oj["package-windows"]); b=dict(nj["package-windows"]); a.pop("steps"); b.pop("steps"); assert a==b'`
- 新: `./openspec/changes/ch17-windows-install-ci-reuse/check-ci-scope.sh`
- 理由: 深掘り第 2 回 Q3 の答えで chain の job に 2 step を足した（b5e319f5b）ので、package-windows 以外を 1 字比較する旧コマンドは HEAD で AssertionError（rc=1）。Q3 が許した範囲（chain は merge-base の並びにその 2 step を足しただけ）を assert する台本に移す（review R14）
- 実証: evidence.jsonl の 2026-11-17T05:20:49+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261117-142049-evidence-2.3-3-55d7e5-vqgqqtc9t.log、HEAD c5d1f7f）
- 承認: machine: evidence.jsonl 2026-11-17T05:21:17+00:00 PASS
