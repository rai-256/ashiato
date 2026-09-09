#!/usr/bin/env bash
# 「収集した」記録が書き換えられないことを、**わざと UPDATE を投げて**確かめる
# （製造準備 C の作法 / design D3 / FR-30）。
#
# アプリを通さず psql から直に叩く。**素通りする経路がここだから** ——
# 同じ PC の第三者製プラグインや手作業の psql を止められるのは DB の側だけ。
set -euo pipefail
cd "$(dirname "$0")/.."
cleanup() { docker compose down -v >/dev/null 2>&1 || true; }
trap cleanup EXIT

psql() { docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato "$@"; }

echo "== DB を起動してマイグレーションを当てる"
docker compose up -d --wait db >/dev/null
psql < migrations/0001_envelope.sql >/dev/null
psql < migrations/0002_immutable_collected.sql >/dev/null
psql < migrations/0003_raw_text.sql >/dev/null
psql < migrations/0004_immutable_origin.sql >/dev/null

# **2 回当てても壊れないことを、ここで確かめる**（review R12）。
# run() は起動のたびに全版を当てるので、当て直しが安全でないと 2 回目の起動で落ちる。
# 0003 は「型が text なら何もしない」分岐を持っているが、その分岐を通る検査がどこにも無かった。
echo "== もう一度当てる（run() は起動のたびに全版を当てる）"
for m in 0001_envelope 0002_immutable_collected 0003_raw_text 0004_immutable_origin; do
  psql < "migrations/$m.sql" >/dev/null || { echo "  NG $m の 2 回目が落ちた"; exit 1; }
done
echo "  OK 4 版とも当て直せる"

echo "== 「収集した」記録を 1 件置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
         VALUES ('immutable-check','書き換え禁止の確認用',21600) ON CONFLICT DO NOTHING;"  >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('22222222-2222-4222-8222-222222222222',
                 '00000000-0000-0000-0000-000000000000','immutable-check','collected',
                 '2026-09-08T02:00:00Z',540,'Asia/Tokyo',1,'immutable-check-hash',
                 '{\"hello\":\"world\"}','{\"hello\":\"world\"}');" >/dev/null

# Scenario: 収集した記録は書き換えられない
fail=0
# 拒まれるべき 3 つ。**それぞれ別に確かめる** —— 1 つだけ効いていて他が素通しでも気付くように
for col in raw payload event_time; do
  case "$col" in
    event_time) val="'2000-01-01T00:00:00Z'" ;;
    *)          val="'{\"hello\":\"tampered\"}'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE logical_source = 'immutable-check';" \
       >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（FR-30 違反）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done

# 中身が本当に変わっていないこと（トリガが例外を投げても書けていた、を潰す）。
# **原文は text なので `raw->>` は引けない**（0003 / design D16）。丸ごと比べる。
got=$(psql -c "SELECT raw FROM core.event WHERE logical_source = 'immutable-check';")
[ "$got" = '{"hello":"world"}' ] || { echo "  NG 原文が変わっている: $got"; fail=1; }

# **表記だけ違う原文への書き換えも拒む。** 0003 で原文が text になって初めて手に入った保証で、
# jsonb の頃は `{"hello": "world"}`（空白違い）が「同値」と見なされて素通りしていた（review R20）。
if psql -c "UPDATE core.event SET raw = '{\"hello\": \"world\"}'
            WHERE logical_source = 'immutable-check';" >/dev/null 2>&1; then
  echo "  NG 表記だけ違う原文への書き換えが通った（0003 の効き目が消えている）"; fail=1
else
  echo "  OK 表記だけ違う原文への書き換えも拒まれた"
fi

# **由来を経由した迂回を塞げているか**（review R2 / design D21 / 0004）。
# 1 手ずつ投げているだけでは見えない経路 —— 'authored' へ移してから書き換え、'collected' へ戻す。
if psql -c "UPDATE core.event SET origin = 'authored'
            WHERE logical_source = 'immutable-check' AND origin = 'collected';" >/dev/null 2>&1; then
  echo "  NG 収集した記録の由来を動かせた（原文の不変が 3 手で迂回できる）"; fail=1
else
  echo "  OK 収集した記録の由来は動かせない"
fi
# 来歴（冪等キー・格納の時刻）も凍結されている
for col in content_hash ingest_time; do
  case "$col" in
    ingest_time) val="'2000-01-01T00:00:00Z'" ;;
    *)           val="'rewritten'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE logical_source = 'immutable-check';" \
       >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（来歴が動く）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done
# 最後に、3 手を通しで打っても原文が変わっていないことを見る
psql -c "UPDATE core.event SET origin='authored' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
psql -c "UPDATE core.event SET raw='{\"tampered\":1}' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
psql -c "UPDATE core.event SET origin='collected' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
got=$(psql -c "SELECT origin||' | '||raw FROM core.event WHERE logical_source='immutable-check';")
[ "$got" = 'collected | {"hello":"world"}' ] \
  || { echo "  NG 3 手の迂回で原文が変わった: $got"; fail=1; }
echo "  OK 3 手を通しても原文は変わらない"

# 論理削除は通らないといけない（tasks 3.2 / FR-50）
if psql -c "UPDATE core.event SET deleted_at = now(), deleted_by = 'check'
            WHERE logical_source = 'immutable-check';" >/dev/null 2>&1; then
  echo "  OK 論理削除は通る"
else
  echo "  NG 論理削除まで止めている（FR-50 を壊している）"; fail=1
fi

# 「本人が書いた」記録は書き換えてよい（禁止の範囲が広がっていないこと）
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('33333333-3333-4333-8333-333333333333',
                 '00000000-0000-0000-0000-000000000000','immutable-check','authored',
                 '2026-09-08T03:00:00Z',540,'Asia/Tokyo',1,'authored-hash','{}','{}');" >/dev/null
if psql -c "UPDATE core.event SET payload = '{\"edited\":true}'
            WHERE origin = 'authored';" >/dev/null 2>&1; then
  echo "  OK 本人が書いた記録は書き換えられる"
else
  echo "  NG 収集以外まで止めている"; fail=1
fi

[ "$fail" -eq 0 ] && echo "書き換え禁止 OK" || { echo "書き換え禁止 NG"; exit 1; }
