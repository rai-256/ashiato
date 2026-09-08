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

echo "縦串 OK"
