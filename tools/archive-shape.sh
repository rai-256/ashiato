#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# 確認待ちの書庫の形だけを表示・確認する（値や原文は表示しない）。
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL が必要です}"
: "${ASHIATO_ARCHIVE_USER_ID:?ASHIATO_ARCHIVE_USER_ID が必要です}"

if [[ "${1:-}" == "--confirm" ]]; then
  shift
  [[ $# -gt 0 ]] || { echo "確認する形のハッシュが必要です" >&2; exit 2; }
  # **印は形ごとに 1 行**（追記のみの表で一意制約が無い）。確認待ちのファイルの数だけ
  # 足していたときは、同じ形の印が何行も積まれた（R49）。
  for shape in "$@"; do
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -v user="$ASHIATO_ARCHIVE_USER_ID" -v shape="$shape" <<'SQL'
INSERT INTO core.archive_shape_confirmation (user_id, shape_hash, shape)
SELECT :'user'::uuid, :'shape', p.shape
FROM core.archive_pending_shape p
WHERE p.user_id = :'user'::uuid AND p.shape_hash = :'shape'
  AND NOT EXISTS (SELECT 1 FROM core.archive_shape_confirmation c
                   WHERE c.user_id = :'user'::uuid AND c.shape_hash = :'shape')
LIMIT 1;
SQL
  done
  exit 0
fi

# **`-c` では `:'user'` が展開されない**（psql の変数は入力の台本の中でだけ展開される）。
# `-c` で渡していたときは引数なしの一覧が必ず構文エラーで落ちた（final review R49）。
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -v user="$ASHIATO_ARCHIVE_USER_ID" <<'SQL'
SELECT shape_hash, shape, count(*) AS files
FROM core.archive_pending_shape
WHERE user_id = :'user'::uuid
GROUP BY shape_hash, shape
ORDER BY shape_hash;
SQL
