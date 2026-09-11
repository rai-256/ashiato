#!/usr/bin/env bash
# 依存の許諾が AGPL-3.0 と両立するかを機械で確かめる（製造準備 C）。
# **本体を公開する = 配布が発生する**ので、ここが赤くなったら入れてはいけない。
set -euo pipefail
cd "$(dirname "$0")/.."
# 許可する許諾。いずれも AGPL-3.0 での配布と両立する（表示義務は満たす）。
# **MIT-0 は ST02 で足した**（`@csstools/*`。vitest の jsdom が引く）。
# MIT から**表示義務すら外した**もので、MIT を許している以上これを断る理由が無い。
ALLOW="${ALLOW_LICENSES:-MIT,MIT-0,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,Unicode-3.0,Unicode-DFS-2016,Zlib,CC0-1.0,MPL-2.0,0BSD,AGPL-3.0-only,Apache-2.0 WITH LLVM-exception,Unicode-3.0,CDLA-Permissive-2.0,BlueOak-1.0.0,CC-BY-4.0,Python-2.0}"
fail=0

echo "== Rust の依存"
cargo metadata --format-version 1 --all-features > /tmp/meta.json
python3 - "$ALLOW" <<'PY'
import json, sys, re
allow = {a.strip() for a in sys.argv[1].split(",")}

def ok(expr, allow):
    """SPDX の式を評価する。AND は全部、OR はどれか 1 つが許可されていればよい。"""
    toks = re.findall(r"\(|\)|[A-Za-z0-9.\-+]+(?:\s+WITH\s+[A-Za-z0-9.\-+]+)?|AND|OR", expr)
    pos = 0
    def parse_or():
        nonlocal pos
        v = parse_and()
        while pos < len(toks) and toks[pos] == "OR":
            pos += 1
            v = parse_and() or v
        return v
    def parse_and():
        nonlocal pos
        v = parse_atom()
        while pos < len(toks) and toks[pos] == "AND":
            pos += 1
            v = parse_atom() and v
        return v
    def parse_atom():
        nonlocal pos
        if pos < len(toks) and toks[pos] == "(":
            pos += 1
            v = parse_or()
            if pos < len(toks) and toks[pos] == ")":
                pos += 1
            return v
        t = toks[pos] if pos < len(toks) else ""
        pos += 1
        return t in allow
    return bool(toks) and parse_or()

meta = json.load(open("/tmp/meta.json"))
bad = []
for p in meta["packages"]:
    lic = (p.get("license") or "").strip()
    if not lic:
        bad.append((p["name"], "(許諾の記載が無い)")); continue
    # "MIT OR Apache-2.0" のような選択は、1 つでも許可されていれば通る
    if not ok(lic.replace("/", " OR "), allow):
        bad.append((p["name"], lic))
for n, l in bad:
    print(f"  NG {n}: {l}")
print(f"  {len(meta['packages'])} 件を確認 / 不許可 {len(bad)} 件")
sys.exit(1 if bad else 0)
PY
fail=$(( fail + $? ))

echo "== Node の依存"
python3 - "$ALLOW" <<'PY'
import json, os, sys, re
allow = {a.strip() for a in sys.argv[1].split(",")}

def ok(expr, allow):
    """SPDX の式を評価する。AND は全部、OR はどれか 1 つが許可されていればよい。"""
    toks = re.findall(r"\(|\)|[A-Za-z0-9.\-+]+(?:\s+WITH\s+[A-Za-z0-9.\-+]+)?|AND|OR", expr)
    pos = 0
    def parse_or():
        nonlocal pos
        v = parse_and()
        while pos < len(toks) and toks[pos] == "OR":
            pos += 1
            v = parse_and() or v
        return v
    def parse_and():
        nonlocal pos
        v = parse_atom()
        while pos < len(toks) and toks[pos] == "AND":
            pos += 1
            v = parse_atom() and v
        return v
    def parse_atom():
        nonlocal pos
        if pos < len(toks) and toks[pos] == "(":
            pos += 1
            v = parse_or()
            if pos < len(toks) and toks[pos] == ")":
                pos += 1
            return v
        t = toks[pos] if pos < len(toks) else ""
        pos += 1
        return t in allow
    return bool(toks) and parse_or()

root = "web/node_modules"
bad, n = [], 0
for dirpath, dirnames, filenames in os.walk(root):
    if "package.json" not in filenames or os.path.basename(os.path.dirname(dirpath)) == "node_modules" and False:
        continue
    try:
        pkg = json.load(open(os.path.join(dirpath, "package.json"), encoding="utf-8"))
    except Exception:
        continue
    if "name" not in pkg or "version" not in pkg:
        continue
    n += 1
    lic = pkg.get("license") or ""
    if isinstance(lic, dict):
        lic = lic.get("type", "")
    lic = str(lic).strip()
    if not lic:
        bad.append((pkg["name"], "(許諾の記載が無い)")); continue
    if not ok(lic, allow):
        bad.append((pkg["name"], lic))
for x, l in bad[:20]:
    print(f"  NG {x}: {l}")
print(f"  {n} 件を確認 / 不許可 {len(bad)} 件")
sys.exit(1 if bad else 0)
PY
fail=$(( fail + $? ))

# --- Android（gradle / maven）は**この検査の対象外**（review R5）
# 黙っていると、緑が「Android の依存も確認済み」に読まれる。実際には 1 件も見ていない。
# `com.google.android.gms:play-services-location` はプロプライエタリ（Android SDK Terms）で、
# **AGPL-3.0 での公開と噛み合うかは未決**（design D15 / Open Questions）。
echo "== Android の依存"
echo "  対象外。gradle / maven は見ていない（design D15 で公開前に決める）"
echo "  試験だけの依存（robolectric: Apache-2.0 / androidx.test: Apache-2.0）は配布物に入らない"

[ "$fail" -eq 0 ] && echo "ライセンス OK" || { echo "ライセンス NG"; exit 1; }
