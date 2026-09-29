#!/usr/bin/env bash
# DB の役割を整える（ST28 / design D4）。**何度走らせても同じ結果**になる。
#
#   tools/db-roles.sh               # .env を読む（無ければ環境変数）
#   ENV_FILE=path tools/db-roles.sh # 別の環境ファイルを読む
#
# やること（管理者 = POSTGRES_USER。コンテナの中の psql で入るので合言葉は要らない）:
#   (a) 管理者の合言葉を POSTGRES_PASSWORD に揃える
#   (b) ashiato_owner（移行・所有）と ashiato_app（サーバの実行時）を作る／合言葉を揃える
#   (c) DB と core の schema・表・sequence・関数・型の所有を ashiato_owner へ移す
#       （`REASSIGN OWNED BY` は使わない。管理者が持つ他のものまで動かすので）
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
  psql -X -q -v ON_ERROR_STOP=1 -v "admin=$admin" -v "dbname=$dbname" -U "$admin" -d "$dbname" >/dev/null <<'SQL'
\getenv admin_pw ADMIN_PW
\getenv owner_pw OWNER_PW
\getenv app_pw APP_PW

SELECT format('ALTER ROLE %I PASSWORD %L', :'admin', :'admin_pw') \gexec

SELECT 'CREATE ROLE ashiato_owner LOGIN'
 WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_owner') \gexec
SELECT format('ALTER ROLE ashiato_owner LOGIN CREATEDB NOSUPERUSER NOCREATEROLE PASSWORD %L', :'owner_pw') \gexec

SELECT 'CREATE ROLE ashiato_app LOGIN'
 WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_app') \gexec
SELECT format('ALTER ROLE ashiato_app LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD %L', :'app_pw') \gexec

SELECT format('ALTER DATABASE %I OWNER TO ashiato_owner', :'dbname') \gexec

-- core の所有を移す。拡張が持つものは動かさない（deptype 'e'）。
-- 表に従属する sequence（serial / identity）は表と一緒に動くので、単独のものだけ。
DO $$
DECLARE r record;
BEGIN
  IF EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'core') THEN
    ALTER SCHEMA core OWNER TO ashiato_owner;
  END IF;
  FOR r IN
    SELECT c.oid, c.relkind, format('%I.%I', n.nspname, c.relname) AS name
      FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'core' AND c.relkind IN ('r','p','v','m','f','S')
       AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = c.oid AND d.deptype IN ('a','i','e'))
  LOOP
    EXECUTE format('ALTER %s %s OWNER TO ashiato_owner',
      CASE r.relkind WHEN 'S' THEN 'SEQUENCE' WHEN 'v' THEN 'VIEW' WHEN 'm' THEN 'MATERIALIZED VIEW'
                     WHEN 'f' THEN 'FOREIGN TABLE' ELSE 'TABLE' END, r.name);
  END LOOP;
  FOR r IN
    SELECT p.prokind, p.oid::regprocedure AS name
      FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
     WHERE n.nspname = 'core'
       AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = p.oid AND d.deptype = 'e')
  LOOP
    EXECUTE format('ALTER %s %s OWNER TO ashiato_owner',
      CASE r.prokind WHEN 'p' THEN 'PROCEDURE' WHEN 'a' THEN 'AGGREGATE' ELSE 'FUNCTION' END, r.name);
  END LOOP;
  FOR r IN
    SELECT format('%I.%I', n.nspname, t.typname) AS name
      FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace
     WHERE n.nspname = 'core' AND t.typtype IN ('e','d','r')
       AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = t.oid AND d.deptype = 'e')
  LOOP
    EXECUTE format('ALTER TYPE %s OWNER TO ashiato_owner', r.name);
  END LOOP;
END $$;
SQL

echo "== 役割を整えた（ashiato_owner / ashiato_app。所有は ashiato_owner）"
