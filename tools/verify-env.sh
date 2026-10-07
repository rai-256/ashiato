#!/usr/bin/env bash
# 検証の準備。ハーネス（scripts/verify-run）が**検証コマンドの前に毎回**呼ぶ。何度呼んでも同じ（起動済みなら何もしない）。
# いまやることは 2 つ: この worktree のテスト用 DB を起動して接続できるまで待ち、DB の役割（ST28）を整える。
#   - ポートは tools/ports.sh の worktree ごとの規則（Story を並行して走らせても取り合わない）
#   - 起動するのはこの worktree の compose の DB だけ。他の Story のコンテナには触らない
#   - 準備できなければ非 0 で終わる（ハーネスは BLOCKED_INFRA として記録し、AI の実装ループに流さない）
# 実測 2026-09-30 ST05 10.4: ポートを分けた後、`cargo test --workspace` の DB を起動する係がいなかった
# （それまでは別の Story の DB が 55432 に残っていて、そこに繋がって通っていた）。
set -euo pipefail
cd "$(dirname "$0")/.."
. tools/ports.sh
docker compose up -d --wait db >/dev/null
# --wait は healthcheck を待つ。テストが繋ぐのと同じ口（127.0.0.1:<ポート>）で受け付けるまで確かめる
for _ in $(seq 1 30); do
  if docker compose exec -T db pg_isready -q -h 127.0.0.1 -U ashiato -d ashiato \
     && (exec 3<>"/dev/tcp/127.0.0.1/${ASHIATO_DB_PORT}") 2>/dev/null; then
    # 役割（ashiato_owner / ashiato_app）を整える。テストは所有者で繋ぐ（ST28 / design D4）。何度呼んでも同じ。
    # **毎回やる** —— tools/smoke.sh は後片付けで DB を volume ごと消すので、次の検証の DB には役割が無い
    # （実測 2026-10-07 ST21 11.3: `password authentication failed for user "ashiato_owner"` で 371 本が落ちた）
    tools/db-roles.sh >/dev/null || { echo "DB の役割を整えられない（tools/db-roles.sh。.env の合言葉を見る）" >&2; exit 1; }
    exit 0
  fi
  sleep 1
done
echo "テスト用 DB（127.0.0.1:${ASHIATO_DB_PORT}）に接続できない" >&2
exit 1
