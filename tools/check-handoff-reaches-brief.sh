#!/usr/bin/env bash
# 申し送り（docs/handoff/ST<NN>.md）が、その Story の brief に逐語で載ることを確かめる。
# 使い方: tools/check-handoff-reaches-brief.sh ST23 ch01-st17-review-fixes
# `story_brief.py` は HTML のパスを出すだけで本文は出さないので、書かれた HTML を読む。
#
# 照らすのは、申し送りの中で見出し（`## `）に <needle> を含む節の**見出しと本文の全部**（review/code.md R8）。
# change 名だけを照らすと、見出しだけ載って本文が落ちても通っていた。
# brief は Markdown を HTML に組み直す（`code` → <code>・行を「。」で <li> に分ける・`- ` を落とす）ので、
# 両方から タグ・バッククォート・`#`・行頭の `- `・空白 を除いた文字の並びで比べる。
# 限界: 文字の並びで比べるので、brief が節の行の順を入れ替えたり、別の節の文と混ぜたりしても、並びが同じなら通る。
set -euo pipefail
cd "$(dirname "$0")/.."
sid="$1"
needle="$2"
python3 scripts/story_brief.py "$sid" >/dev/null
python3 -I - "docs/handoff/$sid.md" "docs/briefs/$sid.html" "$needle" <<'PY'
import html, re, sys

md_path, html_path, needle = sys.argv[1:4]
norm = lambda s: re.sub(r"\s+", "", re.sub(r"(?m)^\s*-\s+", "", s).replace("`", "").replace("#", ""))
md = open(md_path, encoding="utf-8").read()
page = norm(html.unescape(re.sub(r"<[^>]+>", "", open(html_path, encoding="utf-8").read())))
sections = [s for s in re.split(r"(?m)^(?=## )", md) if s.startswith("## ") and needle in s.splitlines()[0]]
if not sections:
    sys.exit(f"  NG {md_path} に見出しが {needle} を含む節が無い")
bad = [s.splitlines()[0] for s in sections if norm(s) not in page]
for head in bad:
    print(f"  NG {html_path} に節が逐語で載っていない: {head}")
sys.exit(1 if bad else 0)
PY
