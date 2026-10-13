#!/usr/bin/env bash
# worktree の `.env` が無ければ作る（合言葉は毎回乱数。ST28 / design D19 —— 字面の合言葉を repo に置かない）。
#
#   tools/env-init.sh     この台本の worktree に作る（port は tools/ports.sh の worktree の規則）。`.env` があれば何もしない（上書きしない）
#
# 実測 2026-10-12: ST17 の下流の worktree に `.env` が無く、`tools/verify-env.sh` の `docker compose up` が
# POSTGRES_PASSWORD 無しで落ち、DB を使う検証が全部 BLOCKED_INFRA になった。他の worktree の `.env` は手で置かれていた。
# DB は worktree ごとの compose project なので、合言葉は worktree ごとに作ってよい
# （既に volume がある worktree に別の合言葉の `.env` を作ると繋がらないので、ある `.env` は触らない）。
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
[ -f "$root/.env" ] && exit 0
. "$(dirname "$0")/ports.sh"
r() { openssl rand -hex 24; }
pg="$(r)" owner="$(r)" app="$(r)"
umask 077
cat > "$root/.env" <<ENV
POSTGRES_PASSWORD=$pg
OWNER_DB_PASSWORD=$owner
APP_DB_PASSWORD=$app
DATABASE_URL=postgres://ashiato_app:$app@127.0.0.1:${ASHIATO_DB_PORT}/ashiato
DATABASE_OWNER_URL=postgres://ashiato_owner:$owner@127.0.0.1:${ASHIATO_DB_PORT}/ashiato
BIND=127.0.0.1:${ASHIATO_HTTP_PORT}
API_TOKEN=$(r)
WEB_PASSWORD=$(r)
ASHIATO_USER_ID=00000000-0000-0000-0000-000000000000
ALLOWED_HOSTS=
ENV
echo "env-init: $root/.env を作った（合言葉は乱数）" >&2
