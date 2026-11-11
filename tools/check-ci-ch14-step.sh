#!/usr/bin/env bash
# package-windows に、インストーラの ch14_ の試験を下限つきで走らせる step が、最初の package.sh windows より前にあるか（CH14 D9）。
# 下限を数える `-- --list` の字面だけで満たさないよう、run を `;` `&&` `||` 改行で区切った片のうち `--list` を含まない片に
# `cargo test -p ashiato-setup ch14_` があること（試験を実際に走らせる呼び出し）まで見る（review/code.md R3・deep.md Q7）
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
import re, sys, yaml
steps = yaml.safe_load(open(".github/workflows/ci.yml", encoding="utf-8"))["jobs"]["package-windows"]["steps"]
r = [s.get("run", "") for s in steps]
def runs_tests(x):
    return any("cargo test -p ashiato-setup ch14_" in p and "--list" not in p for p in re.split(r";|&&|\|\||\n", x))
i = [n for n, x in enumerate(r) if "ashiato-setup ch14_" in x and "-ge 11" in x and runs_tests(x)]
k = [n for n, x in enumerate(r) if "package.sh windows" in x]
sys.exit(0 if i and k and i[0] < k[0] else 1)
PY
