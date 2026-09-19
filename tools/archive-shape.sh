#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# 確認待ちの書庫の形だけを表示・確認する（値や原文は表示しない）。
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL が必要です}"
: "${ASHIATO_ARCHIVE_USER_ID:?ASHIATO_ARCHIVE_USER_ID が必要です}"

if [[ "${1:-}" == "--confirm" ]]; then
  shift
  [[ $# -gt 0 ]] || { echo "確認する形のハッシュが必要です" >&2; exit 2; }
  for shape in "$@"; do
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -v user="$ASHIATO_ARCHIVE_USER_ID" -v shape="$shape" <<'SQL'
INSERT INTO core.archive_shape_confirmation (user_id, shape_hash, shape)
SELECT :'user'::uuid, :'shape', p.shape
FROM core.archive_pending_shape p
WHERE p.user_id = :'user'::uuid AND p.shape_hash = :'shape'
ON CONFLICT DO NOTHING;
SQL
  done
  exit 0
fi

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -v user="$ASHIATO_ARCHIVE_USER_ID" -c \
  "SELECT shape_hash, shape, count(*) AS files FROM core.archive_pending_shape WHERE user_id = :'user'::uuid GROUP BY shape_hash, shape ORDER BY shape_hash"
