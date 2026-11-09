#!/usr/bin/env python3
"""CH13 design D8: `host_only` が harness2 と同じ判定で当たる・当たらないことを確かめる。

    check-workbox-match.py                    tools/workbox.json を match-cases.json の表に当てる
    check-workbox-match.py --request          docs/changes/CH13.md「依頼（逐語）」の JSON に当てる（足した行が落ちることの確認）
    check-workbox-match.py --list <出力>      既存の openspec/**/tasks.md の検証コマンドをホスト / 箱に分けて書き出す

判定は `.harness2/scripts/workbox.host_only` だけ。正規表現をここに写さない。
表のほかに、`docker_images` の各項が `docker-compose.yml` の `image:` に在ることも見る（compose の版を上げて
`docker_images` を直し忘れると、箱が先に渡すイメージが使われず、DB を立てるたびに pull に戻る。review/code.md R2）。
rc: 0 = 表と一致 / 3 = 表との不一致・compose との不一致 / 1 = 読めない・例外などそのほかの失敗
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

CHANGE = Path(__file__).resolve().parent
# 根は `.harness2` を持つ最初の親（archive の後は openspec/changes/archive/<日付>-<change>/ に移るので、段数で決めない。R6）
ROOT = next((p for p in CHANGE.parents if (p / ".harness2" / "scripts").is_dir()), None)
if ROOT is None:
    sys.exit(f"error: {CHANGE} の親に .harness2/scripts が無い")
sys.path.insert(0, str(ROOT / ".harness2" / "scripts"))
import evidence  # noqa: E402
import workbox  # noqa: E402

RC_MISMATCH = 3


def route(cfg: dict, cmd: str) -> str:
    return "host" if workbox.host_only(cfg, cmd) else "box"


def request_config() -> dict:
    """CH13.md の「依頼（逐語）」の ```json の塊。tasks 1.2 と同じ取り出し（行頭の `> ?` を外して正規表現。塊はちょうど 1 つ）。"""
    t = (ROOT / "docs/changes/CH13.md").read_text(encoding="utf-8").split("## 依頼（逐語）", 1)[1].split("\n## ", 1)[0]
    t = "\n".join(re.sub(r"^> ?", "", ln) for ln in t.splitlines())
    blocks = re.findall(r"`{3}json\n(.*?)\n`{3}", t, re.S)
    if len(blocks) != 1:
        raise ValueError(f"docs/changes/CH13.md の「依頼（逐語）」の json の塊が {len(blocks)} 個（1 つのはず）")
    return json.loads(blocks[0])


def compose_mismatch(cfg: dict) -> list[str]:
    """`docker_images` のうち `docker-compose.yml` の `image:` に無いもの。"""
    text = (ROOT / "docker-compose.yml").read_text(encoding="utf-8")
    images = set(re.findall(r"^\s*image:\s*[\"']?([^\s\"'#]+)", text, re.M))
    return [i for i in cfg.get("docker_images", []) if i not in images]


def check(cfg: dict) -> int:
    cases = json.loads((CHANGE / "match-cases.json").read_text(encoding="utf-8"))["cases"]
    bad = [(c, got) for c in cases if (got := route(cfg, c["cmd"])) != c["want"]]
    for c, got in bad:
        print(f"MISMATCH want={c['want']} got={got}: {c['cmd']}\n  why: {c.get('why', '')}")
    print(f"{len(cases) - len(bad)}/{len(cases)} 行が表と一致")
    stale = compose_mismatch(cfg)
    for i in stale:
        print(f"MISMATCH docker_images の {i} が docker-compose.yml の image: に無い（compose の版を上げたら docker_images も直す）")
    return RC_MISMATCH if bad or stale else 0


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
