#!/usr/bin/env bash
# 人間が 1 コマンドで動かすための入口。**依存の導入を前段に含める**
# （依存を足した直後の起動失敗を構造的に塞ぐ / 製造準備 B）。
set -euo pipefail
cd "$(dirname "$0")/.."
. tools/ports.sh     # worktree ごとのポート（Story を並行して走らせても取り合わない。ST05）
# 合言葉は .env の URL から（ST28 / design D19）。port だけをこの worktree のものに差し替える
export DATABASE_URL="$(ashiato_db_url "${DATABASE_URL:?.env を読み込む（set -a; . ./.env; set +a）か DATABASE_URL を渡す}")"
export DATABASE_OWNER_URL="$(ashiato_db_url "${DATABASE_OWNER_URL:?.env を読み込むか DATABASE_OWNER_URL を渡す（移行は所有者の接続で当てる）}")"
export BIND="127.0.0.1:${ASHIATO_HTTP_PORT}"   # port は worktree ごと（.env の BIND の port より優先。ST05）
export API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
export WEB_PASSWORD="${WEB_PASSWORD:?.env を読み込む（set -a; . ./.env; set +a）か WEB_PASSWORD を渡す}"

echo "== 依存を揃える"
cargo fetch -q
(cd web && npm install --silent)

echo "== DB を起動"
docker compose up -d --wait db >/dev/null
./tools/db-roles.sh
echo "== 移行（所有者の接続。当てる前に退避する。既定は db のコンテナの中の pg_dump —— 手元の古い版は db と合わず落ちる。tools/stack.sh）"
export PG_DUMP="${PG_DUMP:-$PWD/tools/pg_dump-docker.sh}"
# 終了コード 3 = DB の版がこの木と合わない。作り直さずに手順を示して止まる（design D14）
migrate_rc=0; cargo run -q -p ashiato-server --bin ashiato-server -- migrate || migrate_rc=$?
if [ "$migrate_rc" = 3 ]; then
  echo "この DB の版の記録がこの木と合わない（上の error: kind= の行が種別。db_ahead なら知らない移行を db: の行に、ledger_unreadable なら記録を読めない理由を出している）。使い捨ての DB なら STACK_RESET=1 で作り直す（tools/stack.sh）。残す DB なら COMPOSE_PROJECT_NAME=<別の名前> で別の DB に立てる（この DB は消えない）" >&2
  exit 3
fi
[ "$migrate_rc" = 0 ] || exit "$migrate_rc"

echo "== サーバと画面を起動（Ctrl-C で両方止まる）"
trap 'kill 0' EXIT
# サーバの環境から管理者と所有者の秘密を外す（design D5）。cargo run は外側で済ませて実体を起動する
cargo build -q -p ashiato-server --bin ashiato-server
env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL ./target/debug/ashiato-server &
(cd web && npm run dev -- --host 127.0.0.1 --port "${DEV_WEB_PORT:-5173}") &
wait
