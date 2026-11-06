#!/usr/bin/env bash
# DB の役割を整える（ST28 / design D4）。**何度走らせても同じ結果**になる。
#
#   tools/db-roles.sh               # .env を読む（無ければ環境変数）
#   ENV_FILE=path tools/db-roles.sh # 別の環境ファイルを読む
#
# やること（管理者 = POSTGRES_USER。コンテナの中の psql で入るので合言葉は要らない）:
#   (a) 管理者の合言葉を POSTGRES_PASSWORD に揃える
#   (b) ashiato_owner（移行・所有）と ashiato_app（サーバの実行時）を作る／合言葉を揃える
#   (b') ashiato_app が持つ役割の付与をすべて外す（所有者の役割を持てば門を外せる）
#   (c) DB と core の schema・表・sequence・関数・型の所有を ashiato_owner へ移す
#       （`REASSIGN OWNED BY` は使わない。管理者が持つ他のものまで動かすので）
#
# **版の検査より前に管理者で走る**ので、版に依らない操作だけにする（design D15）。
# 版に依る操作（表・列・関数を前提にするもの）を足さない。足すなら移行の版にする。
#
# **合言葉は標準出力にも引数にも出さない。** psql へは環境変数で渡し、`\getenv` で読む
# （`-v` だと ps に見える。SQL の文字列に埋めると構文エラーの表示に載る）。
# 雛形（空か change-me で始まる）の合言葉が 1 つでもあれば、DB に触れずに失敗する。
set -euo pipefail
cd "$(dirname "$0")/.."

# 環境ファイルがあれば読む。**ENV_FILE を明示したのに無いときは失敗**（黙って環境変数に倒さない）。
# .env が無くて ENV_FILE も無いときは環境変数のまま（CI は合言葉を $GITHUB_ENV で渡す）。
env_file="${ENV_FILE:-.env}"
if [ -f "$env_file" ]; then set -a; . "$env_file"; set +a
elif [ -n "${ENV_FILE:-}" ]; then echo "error: $ENV_FILE が無い" >&2; exit 1; fi

bad=()
for name in POSTGRES_PASSWORD OWNER_DB_PASSWORD APP_DB_PASSWORD; do
  value="${!name:-}"
  if [ -z "$value" ] || [[ "$value" == change-me* ]]; then bad+=("$name"); fi
done
if [ "${#bad[@]}" -gt 0 ]; then
  echo "error: 雛形のままか空の項目がある。役割は作らない: ${bad[*]}" >&2
  exit 1
fi

admin="${POSTGRES_USER:-ashiato}"
dbname="${POSTGRES_DB:-ashiato}"
export ADMIN_PW="$POSTGRES_PASSWORD" OWNER_PW="$OWNER_DB_PASSWORD" APP_PW="$APP_DB_PASSWORD"

docker compose exec -T -e ADMIN_PW -e OWNER_PW -e APP_PW db \
  psql -X -q -v ON_ERROR_STOP=1 -v "admin=$admin" -v "dbname=$dbname" -U "$admin" -d "$dbname" \
  < tools/db-roles.sql >/dev/null

echo "== 役割を整えた（ashiato_owner / ashiato_app。所有は ashiato_owner）"
