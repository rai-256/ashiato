#!/usr/bin/env bash
# 最短の縦串。ダミーを 1 件入れて → 保存され → 取り出せる、を機能ゼロで通す。
# 部品が個別に動くことしか見ていない他の項目と違い、
# **A で決めたもの同士が噛み合うかを見るのはこの 1 本だけ**（製造準備 B）。
set -euo pipefail
cd "$(dirname "$0")/.."
export DATABASE_URL="${DATABASE_URL:-postgres://ashiato:ashiato@127.0.0.1:55432/ashiato}"
export BIND="${BIND:-127.0.0.1:18787}"
export API_TOKEN="${API_TOKEN:-smoke-token-0123456789abcdef}"
AUTH=(-H "authorization: Bearer $API_TOKEN")

cleanup() { kill "${SRV:-0}" 2>/dev/null || true; docker compose down -v >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "== 1. DB を起動"
docker compose up -d --wait db >/dev/null

echo "== 2. サーバを起動（起動時にマイグレーションを当てる）"
# **ビルドを起動待ちの外に出す。** cargo run のままだと待ち時間の中でコンパイルが走り、
# CI の cold build（2〜4 分）が 60 秒の待ちを超えて「起動しない」と誤判定する
# （実測: GitHub Actions で curl exit 7 / run 34209473342）。
# 生成物を直接起動するので、trap の kill が確実にサーバへ届く利点もある。
cargo build -q -p ashiato-server --bin ashiato-server
./target/debug/ashiato-server & SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$BIND/healthz" >/dev/null

echo "== 3. ソースを登録簿へ 1 行（FR-61: API を変えずにソースを増やす）"
docker compose exec -T db psql -q -U ashiato -d ashiato -c \
  "INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
   VALUES ('smoke','縦串の確認用',21600) ON CONFLICT DO NOTHING;"

echo "== 4. ダミーを 1 件送る"
BODY='{"id":"11111111-1111-4111-8111-111111111111","user_id":"00000000-0000-0000-0000-000000000000",
"logical_source":"smoke","external_id":null,"device_id":"smoke-dev","origin":"collected",
"event_time":"2026-09-08T02:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,
"raw":{"hello":"world"},"payload":{"hello":"world"}}'
R1=$(curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' -d "$BODY")
echo "   → $R1"
echo "$R1" | grep -q '"duplicate":false'

echo "== 5. もう一度同じものを送る（FR-22: 再送しても行が増えない）"
R2=$(curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' -d "$BODY")
echo "   → $R2"
echo "$R2" | grep -q '"duplicate":true'

echo "== 6. 取り出す（論理削除を効かせたビュー越し）"
N=$(curl -sf "${AUTH[@]}" "http://$BIND/events" | grep -o '"id"' | wc -l)
echo "   → $N 件"
[ "$N" -eq 1 ] || { echo "1 件のはずが $N 件"; exit 1; }

echo "== 7. 未登録のソースは受け付けない（登録簿が関門になっている）"
BAD=${BODY/smoke/unknown-source}
code=$(curl -s "${AUTH[@]}" -o /dev/null -w '%{http_code}' -X POST "http://$BIND/ingest" \
  -H 'content-type: application/json' -d "$BAD")
[ "$code" = "400" ] || { echo "400 のはずが $code"; exit 1; }

echo "== 8. バックアップを取り、別の場所へ戻して同じ 1 件が読める（A-3）"
docker compose exec -T db pg_dump -U ashiato -d ashiato > /tmp/ashiato-smoke.sql
docker compose exec -T db psql -q -U ashiato -d postgres -c "CREATE DATABASE restored;"
docker compose exec -T db psql -q -U ashiato -d restored < /tmp/ashiato-smoke.sql >/dev/null
M=$(docker compose exec -T db psql -tA -U ashiato -d restored -c \
  "SELECT count(*) FROM core.event_live;")
echo "   → 復元先に $M 件"
[ "$M" = "1" ] || { echo "復元先が 1 件でない"; exit 1; }

echo "== 9. 合言葉が無い要求は 401（PERM-8: 同じ PC の別プロセスにも素通しさせない）"
code=$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND/events")
[ "$code" = "401" ] || { echo "401 のはずが $code"; exit 1; }



# ここから下は ST01 で足した実データ経路（tasks 8.1）。
# **上の 1〜9 は骨格の縦串**（ダミー 1 件）。下は位置の記録そのものを通す。

psql() { docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato "$@"; }
post()  { curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body \
            -w '%{http_code}' -X POST "http://$BIND/ingest" -d "$1"; }

echo "== 10. 登録簿に c01-location を 1 行（API を変えずにソースを増やす）"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
         VALUES ('c01-location','携帯端末の位置',300) ON CONFLICT DO NOTHING;" >/dev/null

echo "== 11. 位置を 3 件まとめて送る（design D9: まとめ送り）"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
batch='['
for i in 1 2 3; do
  [ "$i" -gt 1 ] && batch="$batch,"
  batch="$batch{\"id\":\"$(printf 'aaaaaaa%d-0000-4000-8000-000000000000' "$i")\",
    \"user_id\":\"00000000-0000-0000-0000-000000000000\",
    \"logical_source\":\"c01-location\",\"external_id\":null,\"device_id\":\"c01-smoke\",
    \"origin\":\"collected\",\"event_time\":\"$NOW\",
    \"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,
    \"unit_system\":\"si\",\"crs\":\"EPSG:4326\",
    \"raw\":{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i * 40 )),\"seq\":$i},
    \"payload\":{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i * 40 ))}}"
done
batch="$batch]"
code=$(post "$batch"); n=$(jq 'length' /tmp/smoke.body)
echo "   → $code / 結果 $n 件"
[ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
[ "$n" = "3" ] || { echo "3 件の結果のはずが $n 件"; exit 1; }   # tasks 5.2
[ "$(jq '[.[]|select(.accepted)]|length' /tmp/smoke.body)" = "3" ] || { echo "3 件とも受け付けられていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event_live WHERE logical_source='c01-location';")" = "3" ] \
  || { echo "3 件入っていない"; exit 1; }

echo "== 12. 水平精度が悪い記録も残る（design D11: 捨てたものは復元できない）"
[ "$(psql -c "SELECT count(*) FROM core.event_live
              WHERE logical_source='c01-location' AND (raw->>'acc_m')::numeric > 100;")" = "1" ] \
  || { echo "精度の悪い 1 件がふるい落とされている"; exit 1; }

echo "== 13. まとめて再送しても行は増えない（FR-22 / tasks 7.5）"
code=$(post "$batch")
echo "   → $code / 重複 $(jq '[.[]|select(.duplicate)]|length' /tmp/smoke.body) 件"
[ "$(jq '[.[]|select(.duplicate)]|length' /tmp/smoke.body)" = "3" ] || { echo "重複と判定されていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event_live WHERE logical_source='c01-location';")" = "3" ] \
  || { echo "再送で行が増えた"; exit 1; }

echo "== 14. 原文は素通し・解析済みは NFC（design D2 / FR-18 / FR-27）"
# "が" を NFD（か U+304B + 濁点 U+3099）で送る。原文は NFD のまま、payload は NFC になるべき。
nfc_body='[{"id":"bbbbbbb1-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000",
  "logical_source":"c01-location","external_id":null,"device_id":"c01-smoke",
  "origin":"collected","event_time":"2026-09-08T04:00:00Z",
  "tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,
  "raw":{"note":"\u304b\u3099"},"payload":{"note":"\u304b\u3099"}}]'
code=$(post "$nfc_body"); [ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
got_raw=$(psql -c "SELECT encode(convert_to(raw->>'note','UTF8'),'hex') FROM core.event
                   WHERE id='bbbbbbb1-0000-4000-8000-000000000000';")
got_pay=$(psql -c "SELECT encode(convert_to(payload->>'note','UTF8'),'hex') FROM core.event
                   WHERE id='bbbbbbb1-0000-4000-8000-000000000000';")
echo "   → 原文 $got_raw / 解析済み $got_pay"
# e3818b e38299 = NFD（か + 濁点）。触っていないこと自体を固定する（tasks 2.2b）
[ "$got_raw" = "e3818be38299" ] || { echo "原文が変換されている（FR-18 違反）"; exit 1; }
# e3818c = NFC（が）
[ "$got_pay" = "e3818c" ] || { echo "解析済みが NFC になっていない（FR-27 違反）"; exit 1; }

echo "== 15. 単位系と座標系（FR-28 / design D4）"
[ "$(psql -c "SELECT crs FROM core.event WHERE id='aaaaaaa1-0000-4000-8000-000000000000';")" = "EPSG:4326" ] \
  || { echo "指定した座標系が入っていない"; exit 1; }
# 単位系を省き、座標系だけ別のものを指定した要求（既定が入ること / 指定が通ること）
alt='[{"id":"ccccccc1-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000",
  "logical_source":"c01-location","external_id":null,"device_id":"c01-smoke",
  "origin":"collected","event_time":"2026-09-08T05:00:00Z",
  "tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,
  "crs":"EPSG:6668",
  "raw":{"note":"plain"},"payload":{"note":"plain"}}]'
code=$(post "$alt"); [ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
row=$(psql -c "SELECT unit_system||' '||crs FROM core.event WHERE id='ccccccc1-0000-4000-8000-000000000000';")
echo "   → $row"
[ "$row" = "si EPSG:6668" ] || { echo "既定の単位系か指定した座標系が入っていない"; exit 1; }

echo "== 16. 一部が不正でも正しい分は格納される（design D9 / tasks 5.3）"
mixed='[{"id":"ddddddd1-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-09-08T06:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":{"seq":"ok1"},"payload":{}},
 {"id":"ddddddd2-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"guessed",
  "event_time":"2026-09-08T06:01:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":{"seq":"bad"},"payload":{}},
 {"id":"ddddddd3-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-09-08T06:02:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":{"seq":"ok2"},"payload":{}}]'
code=$(post "$mixed")
echo "   → $code / $(jq -c '[.[].accepted]' /tmp/smoke.body)"
[ "$code" = "200" ] || { echo "一部成功は 200 のはず（$code）"; exit 1; }
[ "$(jq -c '[.[].accepted]' /tmp/smoke.body)" = "[true,false,true]" ] || { echo "受理の並びが違う"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE raw->>'seq' IN ('ok1','ok2');")" = "2" ] \
  || { echo "正しい分が格納されていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE raw->>'seq'='bad';")" = "0" ] \
  || { echo "不正な分が格納されている"; exit 1; }

echo "== 17. 断る要求（design D5 / FR-25 / FR-20）"
# 由来が列挙にない → 400。**受け取った値を応答に載せない**
bad_origin=$(printf '%s' "$mixed" | jq -c '[.[1]]')
code=$(post "$bad_origin"); body=$(cat /tmp/smoke.body)
echo "   → 由来: $code $body"
[ "$code" = "400" ] || { echo "400 のはずが $code"; exit 1; }
case "$body" in *guessed*) echo "受け取った値が応答に反射している（design D5 違反）"; exit 1 ;; esac
[ "$(jq -r '.[0].error' /tmp/smoke.body)" = "unknown_origin" ] || { echo "理由の種別が違う"; exit 1; }
# 時刻の欄を欠く → 400（tasks 2.6）
for miss in event_time tz_offset_min tz_id; do
  code=$(post "$(printf '%s' "$mixed" | jq -c "[.[0]|del(.$miss)]")")
  [ "$code" = "400" ] || { echo "$miss を欠いたのに $code"; exit 1; }
done
echo "   → 時刻・地域の欄を欠いた 3 通りとも 400"
[ "$(psql -c "SELECT count(*) FROM core.event WHERE id='ddddddd1-0000-4000-8000-000000000000';")" = "1" ] \
  || { echo "断った要求で行が増減している"; exit 1; }

echo "== 18. 生成から格納まで 1 時間以内（NFR-1 / tasks 8.5）"
lag=$(psql -c "SELECT max(abs(extract(epoch from (ingest_time - event_time))))::int
               FROM core.event WHERE id::text LIKE 'aaaaaaa%';")
echo "   → 最大 ${lag} 秒"
[ "$lag" -lt 3600 ] || { echo "1 時間を超えている"; exit 1; }

echo "== 19. 稼働記録が取り込みと同じ関門で立つ（FR-33）"
cov=$(psql -c "SELECT state||' '||event_count FROM core.coverage WHERE logical_source='c01-location';")
echo "   → $cov"
# 3 + NFC 1 + 既定 1 + 混在の 2 = 7。**再送分は数えない**（design D13）
[ "$cov" = "alive 7" ] || { echo "稼働記録の件数が合わない（再送を数えていないか）"; exit 1; }

echo "== 20. 実データ経路が読み出し口から見える（完了の判定 1 行目）"
[ "$(curl -sf "${AUTH[@]}" "http://$BIND/events" | jq '[.[]|select(.logical_source=="c01-location")]|length')" = "7" ] \
  || { echo "読み出し口に 7 件見えない"; exit 1; }

echo "縦串 OK（実データ経路まで）"
