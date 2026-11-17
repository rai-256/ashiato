#!/usr/bin/env bash
# CH17 tasks 2.3: ci.yml で変わったのが Global Constraints の許す範囲だけであることを、merge-base と比べて確かめる（review R14）。
#   - package-windows: step だけが変わってよい（job の if:・needs:・runs-on: などは同じ）
#   - chain: merge-base の step の並びに、深掘り第 2 回 Q3 の 2 step（--self-test と --check-workflow）を
#            続けて足しただけ（他の step は 1 字も変わらず、並びも同じ。job の step 以外も同じ）
#   - それ以外の job と、jobs の外: 1 字も変わらない
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
base="$(git merge-base HEAD origin/main)"
git show "$base:.github/workflows/ci.yml" | python3 -I -c '
import sys, yaml
o = yaml.safe_load(sys.stdin)
n = yaml.safe_load(open(".github/workflows/ci.yml", encoding="utf-8"))
oj, nj = o.pop("jobs"), n.pop("jobs")
fails = []
if o != n:
    fails.append("jobs の外が変わった")
if oj.keys() != nj.keys():
    fails.append("job の集まりが変わった")
for k in oj:
    if k in ("package-windows", "chain") or k not in nj:
        continue
    if oj[k] != nj[k]:
        fails.append("job %s が変わった" % k)
for k in ("package-windows", "chain"):
    a, b = dict(oj[k]), dict(nj[k])
    a.pop("steps"); b.pop("steps")
    if a != b:
        fails.append("job %s の step の外が変わった" % k)
added = [{"run": "./tools/release/windows-build-key.sh --self-test"},
         {"run": "./tools/release/windows-build-key.sh --check-workflow"}]
os_, ns = oj["chain"]["steps"], nj["chain"]["steps"]
at = [i for i in range(len(ns) - 1) if ns[i:i + 2] == added]
if len(at) != 1 or ns[:at[0]] + ns[at[0] + 2:] != os_:
    fails.append("chain の step が、merge-base の並びに 2 step を足しただけでない")
for f in fails:
    print("NG: " + f)
if fails:
    sys.exit(1)
print("ok: ci.yml の変更は package-windows の step と chain の 2 step だけ")
'
