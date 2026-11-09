#!/usr/bin/env python3
"""CH13 design D8: `host_only` が harness2 と同じ判定で当たる・当たらないことを確かめる。

    check-workbox-match.py                    tools/workbox.json を match-cases.json の表に当てる
    check-workbox-match.py --request          docs/changes/CH13.md「依頼（逐語）」の JSON に当てる（足した行が落ちることの確認）
    check-workbox-match.py --list <出力>      既存の openspec/**/tasks.md の検証コマンドをホスト / 箱に分けて書き出す

判定は `.harness2/scripts/workbox.host_only` だけ。正規表現をここに写さない。
rc: 0 = 表と一致 / 3 = 表との不一致 / 1 = 読めない・例外などそのほかの失敗
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CHANGE = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / ".harness2" / "scripts"))
import evidence  # noqa: E402
import workbox  # noqa: E402

RC_MISMATCH = 3


def route(cfg: dict, cmd: str) -> str:
    return "host" if workbox.host_only(cfg, cmd) else "box"


def request_config() -> dict:
    """CH13.md の「依頼（逐語）」の ```json の塊（行頭の `> ` を外す）。"""
    lines, inside, buf, fenced = (ROOT / "docs/changes/CH13.md").read_text(encoding="utf-8").splitlines(), False, [], False
    for ln in lines:
        if ln.startswith("## 依頼（逐語）"):
            inside = True
            continue
        if inside and ln.startswith("## "):
            break
        if not inside:
            continue
        body = ln[2:] if ln.startswith("> ") else ln.lstrip(">")
        if body.strip() == "```json":
            fenced = True
        elif fenced and body.strip() == "```":
            break
        elif fenced:
            buf.append(body)
    if not buf:
        raise ValueError("docs/changes/CH13.md の「依頼（逐語）」に json の塊が無い")
    return json.loads("\n".join(buf))


def check(cfg: dict) -> int:
    cases = json.loads((CHANGE / "match-cases.json").read_text(encoding="utf-8"))["cases"]
    bad = [(c, route(cfg, c["cmd"])) for c in cases if route(cfg, c["cmd"]) != c["want"]]
    for c, got in bad:
        print(f"MISMATCH want={c['want']} got={got}: {c['cmd']}\n  why: {c.get('why', '')}")
    print(f"{len(cases) - len(bad)}/{len(cases)} 行が表と一致")
    return RC_MISMATCH if bad else 0


def listing(cfg: dict, out: Path) -> None:
    host, box = [], []
    for tm in sorted((ROOT / "openspec").rglob("tasks.md")):
        rel = tm.relative_to(ROOT).as_posix()
        for it in evidence.items(tm):
            for cmd in evidence.declared(it):
                cmd = evidence.unwrap_quiet(cmd)
                line = f"- {rel} {it.id}: {' '.join(cmd.split())}"
                (host if route(cfg, cmd) == "host" else box).append(line)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    parts = ["# 検証コマンドの分類（CH13）", "",
             "既存の `openspec/**/tasks.md`（archive を含む）の検証コマンドを、harness2 と同じ取り出し"
             "（`evidence.items` → `declared` → `unwrap_quiet`）と判定（`workbox.host_only`）で分けた一覧。"
             "`check-workbox-match.py --list` が書き出す。", "",
             f"- 書き出したときの HEAD: {head}",
             f"- ホストで走る（host_only）: {len(host)} 本",
             f"- 箱で走る: {len(box)} 本", "",
             "## ホストで走る（host_only）", "", *host, "", "## 箱で走る", "", *box, ""]
    out.write_text("\n".join(parts), encoding="utf-8")
    print(f"host={len(host)} box={len(box)} -> {out}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--request", action="store_true")
    ap.add_argument("--list", metavar="OUT", type=Path)
    a = ap.parse_args()
    try:
        cfg = request_config() if a.request else json.loads((ROOT / "tools/workbox.json").read_text(encoding="utf-8"))
        if a.list:
            listing(cfg, a.list)
            return 0
        return check(cfg)
    except Exception as e:  # 読めない・例外は 1（表との不一致の 3 と分ける）
        print(f"error: {type(e).__name__}: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
