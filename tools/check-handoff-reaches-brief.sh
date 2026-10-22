#!/usr/bin/env bash
# 申し送り（docs/handoff/ST<NN>.md）が、その Story の brief に逐語で載ることを確かめる。
# 使い方: tools/check-handoff-reaches-brief.sh ST23 ch01-st17-review-fixes
# `story_brief.py` は HTML のパスを出すだけで本文は出さないので、書かれた HTML を読む。
set -euo pipefail
cd "$(dirname "$0")/.."
sid="$1"
needle="$2"
python3 scripts/story_brief.py "$sid" >/dev/null
grep -q -- "$needle" "docs/briefs/$sid.html"
