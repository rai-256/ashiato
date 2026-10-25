#!/usr/bin/env bash
# server の試験を並列度 4 で走らせる（.env の接続先を読む）。
# 既定の並列度（コア数）では、時間の上限を持つ試験（作り直し 3 秒・走査の回数）が負荷に負けて揺れる（CH06 Task 7 の実測）。
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
[ -f .env ] && . ./.env
set +a
exec scripts/quiet-run ct -- cargo test -p ashiato-server -- --test-threads=4
