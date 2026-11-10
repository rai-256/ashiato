#!/usr/bin/env bash
# package-windows に、インストーラの ch14_ の試験を下限つきで走らせる step が、最初の package.sh windows より前にあるか（CH14 D9）
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
import sys, yaml
steps = yaml.safe_load(open(".github/workflows/ci.yml", encoding="utf-8"))["jobs"]["package-windows"]["steps"]
r = [s.get("run", "") for s in steps]
i = [n for n, x in enumerate(r) if "ashiato-setup ch14_" in x and "-ge 11" in x]
k = [n for n, x in enumerate(r) if "package.sh windows" in x]
sys.exit(0 if i and k and i[0] < k[0] else 1)
PY
