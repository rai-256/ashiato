#!/usr/bin/env bash
# worktree の `.env` が無ければ作る（合言葉は毎回乱数。ST28 / design D19 —— 字面の合言葉を repo に置かない）。
#
#   tools/env-init.sh     この台本の worktree に作る（port は tools/ports.sh の worktree の規則）。
#                         合言葉の行は上書きしない。`COMPOSE_FILE` の行と `docker-compose.subnet.yml`（subnet の生成物。追跡しない）だけを今の木の値に保つ
#                         （CI のように合言葉が環境変数にあり `.env` が無いときと、録画の `.env`（`COMPOSE_PROJECT_NAME` の行あり）には何もしない。design D7）
#
# 実測 2026-10-12: ST17 の下流の worktree に `.env` が無く、`tools/verify-env.sh` の `docker compose up` が
# POSTGRES_PASSWORD 無しで落ち、DB を使う検証が全部 BLOCKED_INFRA になった。他の worktree の `.env` は手で置かれていた。
# DB は worktree ごとの compose project なので、合言葉は worktree ごとに作ってよい
# （既に volume がある worktree に別の合言葉の `.env` を作ると繋がらないので、ある `.env` の合言葉の行は触らない）。
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
if [ ! -f "$root/.env" ] && [ -n "${POSTGRES_PASSWORD:-}" ] && [ -n "${OWNER_DB_PASSWORD:-}" ] && [ -n "${APP_DB_PASSWORD:-}" ]; then exit 0; fi
if [ -f "$root/.env" ] && grep -q '^COMPOSE_PROJECT_NAME=' "$root/.env"; then exit 0; fi
. "$(dirname "$0")/ports.sh"

# subnet は port から（環境変数の古い値も .env も見ない）。出なければファイルを作らず COMPOSE_FILE も書かない
subnet="$(ashiato_db_subnet)"
compose_file=
if [ -n "$subnet" ]; then
  compose_file=docker-compose.yml:docker-compose.subnet.yml
  [ -f "$root/docker-compose.override.yml" ] && compose_file=$compose_file:docker-compose.override.yml
  want="# env-init.sh が作る生成物（追跡しない）。この木の DB の network の subnet は tools/ports.sh の ashiato_db_subnet が決める
networks:
  default:
    ipam:
      config:
        - subnet: $subnet"
  if [ "$(cat "$root/docker-compose.subnet.yml" 2>/dev/null)" != "$want" ]; then
    printf '%s\n' "$want" >"$root/docker-compose.subnet.yml"
  fi
else
  rm -f "$root/docker-compose.subnet.yml"
fi

if [ -f "$root/.env" ]; then
  # COMPOSE_FILE の行だけをそろえる（ほかの行は 1 文字も変えない）。一時ファイルから中身だけを入れ替え、権限と inode を保つ
  tmp="$(mktemp "$root/.env.XXXXXX")"
  trap 'rm -f "$tmp"' EXIT
  awk -v cf="$compose_file" '
    /^COMPOSE_FILE=/ { if (cf != "" && !done) { print "COMPOSE_FILE=" cf; done = 1 } next }
    { print }
    END { if (cf != "" && !done) print "COMPOSE_FILE=" cf }' "$root/.env" >"$tmp"
  cmp -s "$tmp" "$root/.env" || cat "$tmp" >"$root/.env"
  exit 0
fi
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
[ -z "$compose_file" ] || echo "COMPOSE_FILE=$compose_file" >> "$root/.env"
echo "env-init: $root/.env を作った（合言葉は乱数）" >&2
