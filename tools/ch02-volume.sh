#!/usr/bin/env bash
# CH02 design D8: 量の試験を 6 通り（行数 × 消した滞在の有無）、使い捨ての DB で走らせ、
# 出力の `kind=export …` の行を出す。終わったら design の「計測の記録」の表に空の欄が無いことを確かめる。
set -euo pipefail
cd "$(dirname "$0")/.."
set -a; . ./.env; set +a
base="${DATABASE_OWNER_URL%/*}"
for rows in 10000 100000 1200000; do
  for erased in 1 0; do
    db="ch02_volume_${rows}_${erased}"
    docker compose exec -T db psql -U "${POSTGRES_USER:-ashiato}" -d postgres -c "CREATE DATABASE $db OWNER ashiato_owner" >/dev/null
    rc=0
    DATABASE_OWNER_URL="$base/$db" ASHIATO_EXPORT_VOLUME_ROWS=$rows ASHIATO_EXPORT_VOLUME_ERASED=$erased \
      cargo test -p ashiato-server export_volume_one_run_reports_time_and_memory -- --nocapture 2>&1 \
      | grep -E 'kind=export|test result' || rc=$?
    docker compose exec -T db psql -U "${POSTGRES_USER:-ashiato}" -d postgres -c "DROP DATABASE $db WITH (FORCE)" >/dev/null
    [ "$rc" = 0 ] || exit "$rc"
  done
done
empty=$(grep -c '| |' openspec/changes/ch02-export-next-record-window/design.md || true)
[ "$empty" = 0 ] || { echo "計測の記録に空の欄がある: $empty"; exit 1; }
