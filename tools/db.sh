#!/usr/bin/env bash
# この worktree の DB を操作する `docker compose`（ポートを tools/ports.sh の規則に揃える）。
#   tools/db.sh up -d --wait db     # テスト用 DB を起動する
#   tools/db.sh down -v             # 落とす
# `docker compose` を直に叩くと既定の 55432 で立ち、Story の worktree のテスト（既定は 55500+NN）が繋がらない。
set -euo pipefail
cd "$(dirname "$0")/.."
. tools/ports.sh
exec docker compose "$@"
