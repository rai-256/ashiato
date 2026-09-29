#!/usr/bin/env bash
# 人間が 1 コマンドで動かすための入口。**依存の導入を前段に含める**
# （依存を足した直後の起動失敗を構造的に塞ぐ / 製造準備 B）。
set -euo pipefail
cd "$(dirname "$0")/.."
export DATABASE_URL="${DATABASE_URL:?.env を読み込む（set -a; . ./.env; set +a）か DATABASE_URL を渡す}"
export DATABASE_OWNER_URL="${DATABASE_OWNER_URL:?.env を読み込むか DATABASE_OWNER_URL を渡す（移行は所有者の接続で当てる）}"
export BIND="${BIND:-127.0.0.1:18787}"
export API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
export WEB_PASSWORD="${WEB_PASSWORD:?.env を読み込む（set -a; . ./.env; set +a）か WEB_PASSWORD を渡す}"

echo "== 依存を揃える"
cargo fetch -q
(cd web && npm install --silent)

echo "== DB を起動"
docker compose up -d --wait db >/dev/null
./tools/db-roles.sh
echo "== 移行（所有者の接続）"
cargo run -q -p ashiato-server --bin ashiato-server -- migrate

echo "== サーバと画面を起動（Ctrl-C で両方止まる）"
trap 'kill 0' EXIT
# サーバの環境から管理者と所有者の秘密を外す（design D5）。cargo run は外側で済ませて実体を起動する
cargo build -q -p ashiato-server --bin ashiato-server
env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL ./target/debug/ashiato-server &
(cd web && npm run dev -- --host 127.0.0.1) &
wait
