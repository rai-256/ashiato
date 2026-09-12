#!/usr/bin/env bash
# 開発用の偽データ生成器（製造準備 A-4）。**全 Story がこれを使う。**
# Story ごとに fixture を作らせると、各自バラバラのデータで動作確認することになる。
#
#   ./tools/seed.sh [normal|max|empty]
#     normal  滞在 9 件ぶんに相当する 1 日（UI の方向が前提にしている通常の量）
#     max     要件上限の 15 件（FR-76 は 1 日 5〜15 件。1 画面に収まらない側の確認用）
#     empty   ソースは登録するがデータを入れない（欠損の意味を確かめる）
set -euo pipefail
cd "$(dirname "$0")/.."
MODE="${1:-normal}"
BIND="${BIND:-127.0.0.1:18787}"
API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
AUTH=(-H "authorization: Bearer $API_TOKEN" -H 'content-type: application/json')

case "$MODE" in
  normal) N=9  ;;
  max)    N=15 ;;
  empty)  N=0  ;;
  *) echo "使い方: $0 [normal|max|empty]"; exit 2 ;;
esac

docker compose exec -T db psql -q -U ashiato -d ashiato -c \
  "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
   VALUES ('seed-location','偽データ（位置）',21600,'none') ON CONFLICT DO NOTHING;"

# **原文は text で送る**（design D16）。JSON の値ではなく文字列にくるむ。
# 「収集した」記録には device_id が要る（design D19）。

for i in $(seq 1 "$N"); do
  hh=$(printf '%02d' $(( 7 + i )))
  curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -d "{
    \"id\":\"$(printf '%08d-0000-4000-8000-000000000000' "$i")\",
    \"user_id\":\"00000000-0000-0000-0000-000000000000\",
    \"logical_source\":\"seed-location\",\"external_id\":null,\"device_id\":\"seed\",
    \"origin\":\"collected\",\"event_time\":\"2026-09-07T${hh}:00:00Z\",
    \"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,
    \"raw\":\"{\\\"lat\\\":35.68,\\\"lon\\\":139.76,\\\"acc_m\\\":$(( 5 + i ))}\",
    \"payload\":{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i ))}}" >/dev/null
done
echo "$MODE: $N 件を入れた"
