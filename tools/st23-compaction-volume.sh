#!/usr/bin/env bash
# 夜の書き直しの時間を、実物の量で測る（ST23 tasks 7.2 / design D11 の反転条件 / spec review R15）。
#
#   tools/st23-compaction-volume.sh
#
# 使い捨ての DB（このスクリプトが作って最後に落とす。`tools/stack.sh` の DB は触らない）に、1 年分
#   位置 60 秒ごと = 525,600 行 / 滞在 1 日 15 件 = 5,475 行 / 滞在の前の版 20 件ずつ = 109,500 行
# を `core.event` と `core.event_version` へ入れる（`raw` は `tools/seed.sh` の位置 1 件と同じ形・大きさ）。
# 1 日分の本文を消してから、サーバと同じ `ashiato_app` の接続で `run_once` を 1 回走らせ、
# 表ごとの秒数と表の大きさを標準出力に出す。`seconds_total` が上限（既定 300 秒）を超えたら rc=1
# （design D11 の反転条件。上流へ返す）。`tools/seed.sh max` は 1 日分で量が足りないので使わない。
set -euo pipefail
cd "$(dirname "$0")/.."
. tools/ports.sh
set -a; . "${ENV_FILE:-.env}"; set +a

LIMIT="${COMPACTION_VOLUME_LIMIT_SEC:-300}"
ADMIN="${POSTGRES_USER:-ashiato}"
DB="st23_compaction_volume_$$"

psql_admin() { docker compose exec -T db psql -X -q -v ON_ERROR_STOP=1 -U "$ADMIN" "$@"; }

cleanup() { psql_admin -d postgres -c "DROP DATABASE IF EXISTS $DB WITH (FORCE)" >/dev/null 2>&1 || true; }
trap cleanup EXIT

# 接続先の DB 名と port だけを差し替える（合言葉は環境から。表示しない）
retarget() { printf '%s' "$1" | sed -E "s#@([^:/]+):[0-9]+/[^/?]*#@\1:${ASHIATO_DB_PORT}/${DB}#"; }
OWNER_URL="$(retarget "$DATABASE_OWNER_URL")"
APP_URL="$(retarget "$DATABASE_URL")"

psql_admin -d postgres -c "CREATE DATABASE $DB OWNER ashiato_owner" >/dev/null
echo "== 移行を当てる"
DATABASE_OWNER_URL="$OWNER_URL" MIGRATE_BACKUP=off cargo run -q -p ashiato-server --bin ashiato-server -- migrate >/dev/null

echo "== 1 年分を入れる"
psql_admin -d "$DB" <<'SQL'
SET session_replication_role = replica;  -- 門（トリガ）を通さず入れる。量を作るためだけの使い捨ての DB
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('c01-location', '位置', 21600, 'none'), ('s01-stay', '滞在', 21600, 'none')
ON CONFLICT DO NOTHING;

-- 位置: 60 秒ごと 1 年分（seed の 1 件と同じ形）
INSERT INTO core.event (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
                        schema_version, content_hash, raw, payload)
SELECT gen_random_uuid(), '00000000-0000-0000-0000-000000000001', 'c01-location', 'collected',
       timestamptz '2025-10-01 00:00:00+09' + i * interval '1 minute', 540, 'Asia/Tokyo', 1,
       md5('loc' || i),
       '{"lat":35.68,"lon":139.76,"acc_m":' || (5 + i % 10) || '}',
       jsonb_build_object('lat', 35.68, 'lon', 139.76, 'acc_m', 5 + i % 10)
  FROM generate_series(0, 525599) AS i;

-- 滞在: 1 日 15 件 × 365 日（raw は位置と同じ大きさ）
INSERT INTO core.event (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
                        schema_version, content_hash, raw, payload)
SELECT gen_random_uuid(), '00000000-0000-0000-0000-000000000001', 's01-stay', 'derived',
       timestamptz '2025-10-01 00:00:00+09' + (i / 15) * interval '1 day' + (i % 15) * interval '90 minutes',
       540, 'Asia/Tokyo', 1, md5('stay' || i),
       '{"lat":35.68,"lon":139.76,"acc_m":' || (5 + i % 10) || '}',
       jsonb_build_object('lat', 35.68, 'lon', 139.76, 'acc_m', 5 + i % 10)
  FROM generate_series(0, 5474) AS i;

-- 滞在の前の版: 1 件につき 20 版
INSERT INTO core.event_version (event_id, user_id, logical_source, version_no, event_time, content_hash,
                                raw, payload, tz_offset_min, tz_id)
SELECT e.id, e.user_id, e.logical_source, v, e.event_time, md5(e.content_hash || v),
       '{"lat":35.68,"lon":139.76,"acc_m":' || (5 + v) || '}',
       jsonb_build_object('lat', 35.68, 'lon', 139.76, 'acc_m', 5 + v), 540, 'Asia/Tokyo'
  FROM core.event e CROSS JOIN generate_series(1, 20) AS v
 WHERE e.logical_source = 's01-stay';

-- 最後の 1 日の本文を消す（消去の形: raw = '' と payload = '{}'）と、その操作の台帳の 1 行
UPDATE core.event SET raw = '', payload = '{}'
 WHERE event_time >= timestamptz '2026-09-30 00:00:00+09' AND event_time < timestamptz '2026-10-01 00:00:00+09';
UPDATE core.event_version v SET raw = '', payload = '{}'
  FROM core.event e
 WHERE e.id = v.event_id AND e.raw = '' AND e.logical_source = 's01-stay';
INSERT INTO core.erasure_operation (id, user_id, kind, stay_id, start_at, end_at, stays, locations,
                                    archive_locations, archive_items, versions)
SELECT gen_random_uuid(), user_id, 'user', id, event_time, event_time + interval '1 hour', 15, 1440, 0, 0, 300
  FROM core.event WHERE logical_source = 's01-stay' AND raw = '' LIMIT 1;
SQL

echo "== 夜の書き直しを 1 回走らせる"
log="$(mktemp)"
trap 'rm -f "$log"; cleanup' EXIT
DATABASE_OWNER_URL="$OWNER_URL" DATABASE_URL="$APP_URL" \
  cargo test -q -p ashiato-server compaction_volume_run_once -- --ignored --nocapture --test-threads=1 2>&1 | tee "$log"
grep -Eq 'test result: ok\. 1 passed' "$log" || { echo "error: 計測の試験が通らなかった" >&2; exit 1; }

total="$(sed -n 's/^seconds_total=//p' "$log")"
[ -n "$total" ] || { echo "error: seconds_total が出なかった" >&2; exit 1; }
echo "limit_seconds=$LIMIT"
if awk -v t="$total" -v l="$LIMIT" 'BEGIN { exit !(t > l) }'; then
  echo "error: seconds_total=$total が上限 $LIMIT 秒を超えた（design D11 の反転条件。上流へ返す）" >&2
  exit 1
fi
