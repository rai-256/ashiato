#!/usr/bin/env bash
# 操作盤の「読み物」（/panel/docs/）を作り直す。harness2 の panel が「作り直す」で呼ぶ
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/reading_brief.py "$@"
