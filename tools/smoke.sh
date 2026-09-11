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

echo "== 1. DB を起動（**まっさらにしてから**）"
# **前提を書かないと再現しない**（ST02 の review/code.md の R14）。
# `cargo test` は本物の DB を使うので、その直後に走らせると手順 6 の件数が合わない。
# 末尾の trap と対にして、先頭でも落とす。
docker compose down -v >/dev/null 2>&1 || true
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

# **登録簿の行を、これから送る記録より前の日付にする**（深掘り 第 8 回 Q29）。
# 収集開始日は「登録簿に行ができた日**以降**」の記録・生存信号からしか引かない ——
# 端末の時計が狂った 1 件で開始日が 1999 年に落ち、二度と戻らないのを防ぐため。
# この縦串は日境界を決め打ちで見るために 2026-03 の日付を送るが、
# 登録簿の行は `now()`（当日）で作られるので、**そのままだと全部が「登録より前」になる**。
# 本番では登録簿の行が先にあって収集が後から始まる（FK がそれを強制している）ので、
# ここで揃えるのが本番と同じ形。
# **`WHERE` を付ける**（review/code-r2.md の H-6 / I6）。付けずに全行を書き換えていたので、
# `DATABASE_URL` の向き先を間違えると**登録簿にしか無い事実を黙って消す**形になっていた
# （`registered_at` は第 8 回 Q29 以降、収集開始日の算出根拠になった列）。
docker compose exec -T db psql -q -U ashiato -d ashiato -c \
  "UPDATE core.source SET registered_at = '2026-01-01T00:00:00+09:00'
    WHERE logical_source IN ('smoke','c01-location','c01-app-usage','c01-photo',
                             'c02-window','c02-browser-history');"

# Scenario: 1 件だけの裸の要求も受け取る
#   （配列に包まずに 1 件だけ送る）
echo "== 4. ダミーを 1 件送る"
BODY='{"id":"11111111-1111-4111-8111-111111111111","user_id":"00000000-0000-0000-0000-000000000000",
"logical_source":"smoke","external_id":null,"device_id":"smoke-dev","origin":"collected",
"event_time":"2026-09-08T02:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,
"raw":"{\"hello\":\"world\"}","payload":{"hello":"world"}}'
R1=$(curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' -d "$BODY")
echo "   → $R1"
# **応答の形まで見る**（design D12: 裸のオブジェクトでは返さない）。
# grep だけだと、互換のために 1 件を裸で返す実装が素通りする
[ "$(printf '%s' "$R1" | jq -r 'type')" = "array" ] || { echo "応答が配列でない"; exit 1; }
[ "$(printf '%s' "$R1" | jq 'length')" = "1" ] || { echo "長さ 1 の配列でない"; exit 1; }
[ "$(printf '%s' "$R1" | jq -r '.[0].duplicate')" = "false" ] || { echo "重複と判定された"; exit 1; }

# Scenario: 再送しても重複しない
echo "== 5. もう一度同じものを送る（FR-22: 再送しても行が増えない）"
R2=$(curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' -d "$BODY")
echo "   → $R2"
echo "$R2" | grep -q '"duplicate":true'

echo "== 6. 取り出す（論理削除を効かせたビュー越し）"
N=$(curl -sf "${AUTH[@]}" "http://$BIND/events" | grep -o '"id"' | wc -l)
echo "   → $N 件"
[ "$N" -eq 1 ] || { echo "1 件のはずが $N 件"; exit 1; }

# Scenario: 未登録のソースは拒否される
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

# Scenario: 資格情報が無いと拒否される
# Scenario: 同じ PC の別プロセスからでも拒否される
#   （curl は同じ PC の**別プロセス**。網の外を止めても、これは PERM-7 では止まらない）
echo "== 9. 合言葉が無い要求は 401（PERM-8: 同じ PC の別プロセスにも素通しさせない）"
# **読み出し口と取り込み口の両方**を見る（tasks 9.9）。片方だけだと、
# 書き込み側が素通しになっても気付かない —— 資格情報が要るのは「すべての API 要求」（PERM-10）
code=$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND/events")
[ "$code" = "401" ] || { echo "/events が 401 のはずが $code"; exit 1; }
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$BIND/ingest" \
  -H 'content-type: application/json' -d "$BODY")
[ "$code" = "401" ] || { echo "/ingest が 401 のはずが $code"; exit 1; }
# 合言葉が違うときも 401（無いときだけ見て済ませない）
code=$(curl -s -H "authorization: Bearer wrong-token-0123456789abcdef" \
  -o /dev/null -w '%{http_code}' "http://$BIND/events")
[ "$code" = "401" ] || { echo "違う合言葉で $code"; exit 1; }
[ "$(curl -sf "${AUTH[@]}" "http://$BIND/events" | grep -o '"id"' | wc -l)" -eq 1 ] \
  || { echo "断ったはずの要求で行が増えている"; exit 1; }



# ここから下は ST01 で足した実データ経路（tasks 8.1）。
# **上の 1〜9 は骨格の縦串**（ダミー 1 件）。下は位置の記録そのものを通す。

psql() { docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato "$@"; }
post()  { curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body \
            -w '%{http_code}' -X POST "http://$BIND/ingest" -d "$1"; }
# 原文は **text** で送る（0003 / design D16）。JSON の値ではなく「文字列」なので、
# 送りたい原文をそのまま JSON 文字列にくるむ。
rawstr() { printf '%s' "$1" | jq -Rs .; }

# Scenario: 登録するだけで受け付けられる
#   （10 で登録簿に 1 行足し、11 が API を変えずに通る）
echo "== 10. 登録簿に c01-location を 1 行（API を変えずにソースを増やす）"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
         VALUES ('c01-location','携帯端末の位置',300) ON CONFLICT DO NOTHING;" >/dev/null

# Scenario: 複数件を 1 回で受け取る
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
    \"raw\":$(rawstr "{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i * 40 )),\"seq\":$i}"),
    \"payload\":{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i * 40 )),\"seq\":$i}}"
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
              WHERE logical_source='c01-location' AND (payload->>'acc_m')::numeric > 100;")" = "1" ] \
  || { echo "精度の悪い 1 件がふるい落とされている"; exit 1; }

echo "== 13. まとめて再送しても行は増えない（FR-22 / tasks 7.5）"
code=$(post "$batch")
echo "   → $code / 重複 $(jq '[.[]|select(.duplicate)]|length' /tmp/smoke.body) 件"
[ "$(jq '[.[]|select(.duplicate)]|length' /tmp/smoke.body)" = "3" ] || { echo "重複と判定されていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event_live WHERE logical_source='c01-location';")" = "3" ] \
  || { echo "再送で行が増えた"; exit 1; }

# Scenario: 原文がそのまま残る
# Scenario: 原文は受け取ったまま保存される
# Scenario: 解析済みの文字列は合成済みで保存される
echo "== 14. 原文は素通し・解析済みは NFC（design D2 / FR-18 / FR-27）"
# "が" を NFD（か U+304B + 濁点 U+3099）で送る。原文は NFD のまま、payload は NFC になるべき。
# **原文は text なので丸ごと比べる**（0003 / design D16）—— 中を引くと DB の解釈が挟まる。
nfd_raw=$'{"note":"\u304b\u3099"}'
nfc_body="[{\"id\":\"bbbbbbb1-0000-4000-8000-000000000000\",
  \"user_id\":\"00000000-0000-0000-0000-000000000000\",
  \"logical_source\":\"c01-location\",\"external_id\":null,\"device_id\":\"c01-smoke\",
  \"origin\":\"collected\",\"event_time\":\"2026-09-08T04:00:00Z\",
  \"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,
  \"raw\":$(rawstr "$nfd_raw"),\"payload\":{\"note\":\"\\u304b\\u3099\"}}]"
code=$(post "$nfc_body"); [ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
got_raw=$(psql -c "SELECT encode(convert_to(raw,'UTF8'),'hex') FROM core.event
                   WHERE id='bbbbbbb1-0000-4000-8000-000000000000';")
got_pay=$(psql -c "SELECT encode(convert_to(payload->>'note','UTF8'),'hex') FROM core.event
                   WHERE id='bbbbbbb1-0000-4000-8000-000000000000';")
want_raw=$(printf '%s' "$nfd_raw" | od -An -tx1 | tr -d ' \n')
echo "   → 原文 $got_raw / 解析済み $got_pay"
# 送ったバイト列とそのまま一致する（tasks 2.2b。**中を引かずに丸ごと**）
[ "$got_raw" = "$want_raw" ] || { echo "原文が変換されている（FR-18 違反）: $want_raw を送った"; exit 1; }
# e3818b e38299（か + 濁点）が原文に残っている＝ NFC 化されていない
case "$got_raw" in *e3818be38299*) : ;; *) echo "原文の濁点が合成されている（FR-18 違反）"; exit 1 ;; esac
# e3818c = NFC（が）
[ "$got_pay" = "e3818c" ] || { echo "解析済みが NFC になっていない（FR-27 違反）"; exit 1; }

# Scenario: 収集側が座標系を指定できる
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
  "raw":"{\"note\":\"plain\"}","payload":{"note":"plain"}}]'
code=$(post "$alt"); [ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
row=$(psql -c "SELECT unit_system||' '||crs FROM core.event WHERE id='ccccccc1-0000-4000-8000-000000000000';")
echo "   → $row"
[ "$row" = "si EPSG:6668" ] || { echo "既定の単位系か指定した座標系が入っていない"; exit 1; }

# Scenario: 一部が不正でも正しい分は格納される
echo "== 16. 一部が不正でも正しい分は格納される（design D9 / tasks 5.3）"
mixed='[{"id":"ddddddd1-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-09-08T06:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"ok1\"}","payload":{"seq":"ok1"}},
 {"id":"ddddddd2-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"guessed",
  "event_time":"2026-09-08T06:01:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"bad\"}","payload":{"seq":"bad"}},
 {"id":"ddddddd3-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-09-08T06:02:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"ok2\"}","payload":{"seq":"ok2"}}]'
code=$(post "$mixed")
echo "   → $code / $(jq -c '[.[].accepted]' /tmp/smoke.body)"
[ "$code" = "200" ] || { echo "一部成功は 200 のはず（$code）"; exit 1; }
[ "$(jq -c '[.[].accepted]' /tmp/smoke.body)" = "[true,false,true]" ] || { echo "受理の並びが違う"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE payload->>'seq' IN ('ok1','ok2');")" = "2" ] \
  || { echo "正しい分が格納されていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE payload->>'seq'='bad';")" = "0" ] \
  || { echo "不正な分が格納されている"; exit 1; }

# Scenario: 分類にない値は拒否される
# Scenario: 時刻の欄が欠けた要求は拒否される
# Scenario: 1 件も受け付けなかったときだけ 400
#   （16 の一部不正は 200。ここは全部不正なので 400）
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

# Scenario: 到達できる間は 1 時間以内に届く
echo "== 18. 生成から格納まで 1 時間以内（NFR-1 / tasks 8.5）"
lag=$(psql -c "SELECT max(abs(extract(epoch from (ingest_time - event_time))))::int
               FROM core.event WHERE id::text LIKE 'aaaaaaa%';")
echo "   → 最大 ${lag} 秒"
[ "$lag" -lt 3600 ] || { echo "1 時間を超えている"; exit 1; }

# Scenario: 並び・重複・表記が保たれる
echo "== 19. 並び・重複・表記が保たれる（tasks 9.3 / spec「原文を構造として解釈し直さない」）"
# **jsonb だと落ちる検査。** 実測でこの型は
#   キー順を辞書順に並べ替え / 重複キーを 1 つに畳み / 1e2 を 100 に展開する。
# text で持っている限り、送ったものがそのまま返る（0003 / design D16）。
weird_raw='{"b":1,"a":2,"a":3,"n":1.100,"m":1e2,"z":"  spaced  "}'
weird="[{\"id\":\"eeeeeee1-0000-4000-8000-000000000000\",
  \"user_id\":\"00000000-0000-0000-0000-000000000000\",
  \"logical_source\":\"c01-location\",\"external_id\":null,\"device_id\":\"c01-smoke\",
  \"origin\":\"collected\",\"event_time\":\"2026-09-08T07:00:00Z\",
  \"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,
  \"raw\":$(rawstr "$weird_raw"),\"payload\":{\"seq\":\"weird\"}}]"
code=$(post "$weird"); [ "$code" = "200" ] || { echo "200 のはずが $code"; exit 1; }
got=$(psql -c "SELECT raw FROM core.event WHERE id='eeeeeee1-0000-4000-8000-000000000000';")
echo "   → $got"
[ "$got" = "$weird_raw" ] || {
  echo "原文が構造として解釈し直されている（FR-18 / spec 違反）"
  echo "  送った: $weird_raw"; echo "  戻った: $got"; exit 1; }
# **この検査が jsonb では落ちることを、同じ DB で確かめる** ——
# 検査そのものが空振りしていないことの担保（製造準備 C の作法）
canon=$(psql -c "SELECT ('$weird_raw')::jsonb::text;")
[ "$canon" != "$weird_raw" ] || { echo "jsonb でも同じ値になる（検査が空振りしている）"; exit 1; }
echo "   → jsonb に通すと \"$canon\" に変わる（だからこの型では持てない）"

# Scenario: 出自の欄がすべて埋まる
# Scenario: 2 つの時刻が両方埋まる
# Scenario: 版と単位が埋まる
echo "== 20. 出自・2 つの時刻・版と単位が「送ったとおりに」残る（tasks 9.7 / spec の 3 Scenario）"
# **NULL を数えても意味が無い**（review R3 / R1）—— 0001 で 12 列が NOT NULL なので、
# 行が在る限り「NULL でない」は必ず真になり、DDL の言い換えにしかならない。
# 見るべきは「**送った値が記録として残っているか**」。
want="00000000-0000-0000-0000-000000000000|c01-location|c01-smoke|collected|1|si|EPSG:4326|Asia/Tokyo|540"
n=$(psql -c "SELECT count(*) FROM core.event
             WHERE logical_source='c01-location'
               AND user_id::text||'|'||logical_source||'|'||device_id||'|'||origin||'|'||
                   schema_version||'|'||unit_system||'|'||crs||'|'||tz_id||'|'||tz_offset_min
                   = '$want';")
total=$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-location';")
echo "   → $n / $total 件が送ったとおり"
# 座標系を変えた 1 件（手順 15）だけが外れる。それ以外は全部一致していなければならない
[ "$n" = "$(( total - 1 ))" ] || { echo "送った値と違うものが保存されている"; exit 1; }
[ "$(psql -c "SELECT crs FROM core.event WHERE id='ccccccc1-0000-4000-8000-000000000000';")" = "EPSG:6668" ] \
  || { echo "指定した座標系が残っていない"; exit 1; }

# 2 つの時刻が**別々の意味**を持っている（同じ値を 2 か所に書いているだけ、を潰す）
[ "$(psql -c "SELECT count(*) FROM core.event
              WHERE logical_source='c01-location' AND ingest_time = event_time;")" = "0" ] \
  || { echo "格納の時刻が出来事の時刻の写しになっている（FR-19 の意味が消えている）"; exit 1; }
# 固定の日付で送った分は、格納の時刻が確かに「あとから」入っている
[ "$(psql -c "SELECT count(*) FROM core.event
              WHERE id='bbbbbbb1-0000-4000-8000-000000000000' AND ingest_time > event_time;")" = "1" ] \
  || { echo "格納の時刻が出来事の時刻より後になっていない"; exit 1; }

echo "== 20b. 「収集した」記録に端末識別子が無ければ断る（spec「どの端末が生成したか」/ review R3）"
# **この検査が無いと手順 20 の印は空振りする** —— device_id は 0001 で nullable なので、
# 省いた要求が 200 で通り NULL で保存されていた（独立検証で実測）
no_dev=$(printf '%s' "$mixed" | jq -c '[.[0]|del(.device_id)|.id="f0000001-0000-4000-8000-000000000000"]')
code=$(post "$no_dev")
echo "   → $code / $(jq -r '.[0].error' /tmp/smoke.body)"
[ "$code" = "400" ] || { echo "400 のはずが $code"; exit 1; }
[ "$(jq -r '.[0].error' /tmp/smoke.body)" = "missing_device_id" ] || { echo "理由の種別が違う"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE id='f0000001-0000-4000-8000-000000000000';")" = "0" ] \
  || { echo "端末識別子の無い記録が格納されている"; exit 1; }
# 「本人が書いた」記録には端末を求めない（禁止の範囲が広がっていないこと）。
# **原文を変える** —— 同じにすると冪等キーが一致して重複になり、
# 「格納された」ことを確かめられない（＝この検査が空振りする）
authored=$(printf '%s' "$no_dev" | jq -c '[.[0]
  | .origin="authored" | .id="f0000002-0000-4000-8000-000000000000"
  | .raw="{\"seq\":\"authored\"}" | .payload={"seq":"authored"}]')
[ "$(post "$authored")" = "200" ] || { echo "本人が書いた記録にまで端末を求めている"; exit 1; }
[ "$(jq -r '.[0].duplicate' /tmp/smoke.body)" = "false" ] || { echo "重複になっている（検査が空振り）"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event
              WHERE id='f0000002-0000-4000-8000-000000000000' AND device_id IS NULL;")" = "1" ] \
  || { echo "端末識別子の無い authored が格納されていない"; exit 1; }

echo "== 20c. DB に格納できない原文を、格納の前に断る（review R11 / R18）"
# **1 件の恒久的な失敗が後続を永久に止めるのを防ぐ。** PostgreSQL の text は U+0000 を
# 格納できず、届くとまとめ送り全体が 500 になって収集側は 1 件も取り除けない
for bad_raw in '""' '"{\u0000}"'; do
  body=$(printf '%s' "$mixed" | jq -c "[.[0]|.raw=$bad_raw|.id=\"f0000003-0000-4000-8000-000000000000\"]")
  code=$(post "$body")
  [ "$code" = "400" ] || { echo "原文 $bad_raw が $code で通った"; exit 1; }
  [ "$(jq -r '.[0].error' /tmp/smoke.body)" = "invalid_raw" ] || { echo "理由の種別が違う"; exit 1; }
done
echo "   → 空の原文と NUL を含む原文の 2 通りとも 400（invalid_raw）"
# **正しい分は道連れにならない。** 不正 1 件を挟んでも他が格納される
mixed_bad=$(printf '%s' "$mixed" | jq -c '[.[0]|.raw=""|.id="f0000004-0000-4000-8000-000000000000"]
  + [.[0]|.id="f0000005-0000-4000-8000-000000000000"|.payload={"seq":"after-poison"}|.raw="{\"seq\":\"after-poison\"}"]')
code=$(post "$mixed_bad")
[ "$code" = "200" ] || { echo "不正 1 件でまとめ送り全体が落ちた（$code）"; exit 1; }
[ "$(jq -c '[.[].accepted]' /tmp/smoke.body)" = "[false,true]" ] || { echo "受理の並びが違う"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE payload->>'seq'='after-poison';")" = "1" ] \
  || { echo "不正な 1 件の後ろが格納されていない"; exit 1; }
echo "   → 不正を挟んでも後続は格納される"

echo "== 20d. 契約から外れた本文でも、応答の形は結果の配列（review R15）"
# 平文を返すと収集側がパースに失敗し、状態符号の意味を失う
for body in '[]' '5'; do
  code=$(post "$body")
  [ "$code" = "400" ] || { echo "本文 $body が $code"; exit 1; }
  [ "$(jq -r 'type' /tmp/smoke.body)" = "array" ] || { echo "本文 $body の応答が配列でない"; exit 1; }
done
echo "   → 空配列と非配列の 2 通りとも 400 で、本文は配列"

# Scenario: 重複は件数に加えない
#   （13 で 3 件を再送しても、ここの件数は増えていない）
echo "== 21. 稼働記録が取り込みと同じ関門で立つ（FR-33）"
# **日をまたいで合計する。** 稼働記録は日ごとに 1 行なので、送った記録が 2 つの日に
# またがると行が 2 本になる。テスト 11 は NFR-1（生成から格納まで 1 時間以内）を測るために
# event_time に「いま」を使い、テスト 14〜16 は固定の 2026-09-08 を使うので、
# **実行する日によって行数が変わる**。行を 1 本と決め打つと、書いた当日しか通らない。
# **ST02 で `state` 列が消えた**（design D2）—— 7 状態は行に焼かず導出する。
# 稼働記録が持つのは「その日に何件入ったか」だけになった。
cov=$(psql -c "SELECT sum(event_count) FROM core.coverage WHERE logical_source='c01-location';")
echo "   → $cov"
# 3 + NFC 1 + 既定 1 + 混在の 2 + 表記 1 + 端末検査 1 + 原文検査 1 = 10。
# **再送分は数えない**（design D13）
[ "$cov" = "10" ] || { echo "稼働記録の件数が合わない（再送を数えていないか）"; exit 1; }

echo "== 21b. 重複だけが届いた日も、稼働していたことは記録される（spec の Scenario 後半）"
# **別の日を 1 つ作る。** 手順 13 の再送は手順 11 と同じ日なので、
# 重複時に稼働記録を立てないようにしても手順 21 は緑のまま通る（review R17）
lone='[{"id":"f0000006-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-01-15T03:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"lone\"}","payload":{"seq":"lone"}}]'
[ "$(post "$lone")" = "200" ] || { echo "1 件目が入らない"; exit 1; }
day=$(psql -c "SELECT event_count FROM core.coverage
               WHERE logical_source='c01-location' AND day='2026-01-15';")
[ "$day" = "1" ] || { echo "その日の稼働記録が 1 件でない: $day"; exit 1; }
# 同じ日に**重複だけ**が届く
[ "$(post "$lone")" = "200" ] || { echo "再送が通らない"; exit 1; }
[ "$(jq -r '.[0].duplicate' /tmp/smoke.body)" = "true" ] || { echo "重複と判定されていない"; exit 1; }
day=$(psql -c "SELECT event_count FROM core.coverage
               WHERE logical_source='c01-location' AND day='2026-01-15';")
echo "   → $day"
# 件数は増えない。**行は残る**（「欠損」と「重複だけ届いた」が区別できなくなる）
[ "$day" = "1" ] || { echo "重複で件数が増えたか、行が消えた: $day"; exit 1; }

echo "== 22. 実データ経路が読み出し口から見える（完了の判定 1 行目）"
[ "$(curl -sf "${AUTH[@]}" "http://$BIND/events" | jq '[.[]|select(.logical_source=="c01-location")]|length')" = "11" ] \
  || { echo "読み出し口に 11 件見えない"; exit 1; }

# **読み出し口越しでも原文がそのまま返る**（review R11）。
# DB を直に引く手順 19 とは別の経路 —— JSON へ載せ直すときに二重にエスケープされうる
got=$(curl -sf "${AUTH[@]}" "http://$BIND/events" \
  | jq -r '.[]|select(.id=="eeeeeee1-0000-4000-8000-000000000000")|.raw')
[ "$got" = "$weird_raw" ] || {
  echo "読み出し口で原文が変わっている"
  echo "  送った: $weird_raw"; echo "  戻った: $got"; exit 1; }
echo "   → 読み出し口越しでも原文はそのまま"

# ================================================================ ST02 の縦串
#
# **記録を 1 件も入れずに生存信号だけを送り、稼働状況がその日を②で返す**（tasks 10.1）。
# これが FR-78 の目的そのもの —— 記録経由でしか確かめないと、
# 「記録が 0 件の日の意味が残る」という当の振る舞いに穴が残る。

hbpost() { curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body \
             -w '%{http_code}' -X POST "http://$BIND/heartbeat" -d "$1"; }

echo "== 23. 日境界が Asia/Tokyo（深掘り Q2 / 既存の欠陥 1）"
# 日本時間の 0 時をまたぐ 2 件。**UTC で切ると両方 03-01 に入る**
jst='[{"id":"d0000001-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-03-01T14:59:59Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"jst-a\"}","payload":{"seq":"jst-a"}},
 {"id":"d0000002-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location",
  "external_id":null,"device_id":"c01-smoke","origin":"collected",
  "event_time":"2026-03-01T15:00:01Z","tz_offset_min":540,"tz_id":"Asia/Tokyo",
  "schema_version":1,"raw":"{\"seq\":\"jst-b\"}","payload":{"seq":"jst-b"}}]'
[ "$(post "$jst")" = "200" ] || { echo "日境界の 2 件が入らない"; exit 1; }
got=$(psql -c "SELECT string_agg(day::text, ',' ORDER BY day) FROM core.coverage
               WHERE logical_source='c01-location' AND day IN ('2026-03-01','2026-03-02');")
echo "   → $got"
[ "$got" = "2026-03-01,2026-03-02" ] || { echo "日が Asia/Tokyo で切れていない: $got"; exit 1; }

# Scenario: 記録が 0 件でも生存信号があれば稼働が残る
echo "== 24. 記録を 1 件も入れずに生存信号だけを送る（FR-78 / tasks 10.1）"
# c01-photo は 0005 が登録簿に置いた Must ソース。**記録は 1 件も送らない**
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-photo';")" = "0" ] \
  || { echo "写真に記録が入っている（この検査が空振りする）"; exit 1; }
beat='[{"id":"e1000001-0000-4000-8000-000000000000",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-photo",
  "device_id":"c01-smoke","emitted_at":"2026-03-01T03:00:00Z",
  "capturable":true,"blockers":[],"attempts":4,"successes":4,
  "raw":"{\"alive\":true}"}]'
code=$(hbpost "$beat"); echo "   → $code / $(jq -c '[.[].accepted]' /tmp/smoke.body)"
[ "$code" = "200" ] || { echo "生存信号が 200 で通らない ($code)"; exit 1; }
[ "$(jq -r '.[0].accepted' /tmp/smoke.body)" = "true" ] || { echo "受け付けられていない"; exit 1; }

# Scenario: 同じ生存信号を 2 回送っても 1 行
echo "== 25. 同じ生存信号を再送しても行は 1 つ（第 4 回 Q13）"
[ "$(hbpost "$beat")" = "200" ] || { echo "再送が通らない"; exit 1; }
[ "$(jq -r '.[0].duplicate' /tmp/smoke.body)" = "true" ] || { echo "重複と判定されていない"; exit 1; }
n=$(psql -c "SELECT count(*) FROM core.heartbeat WHERE logical_source='c01-photo';")
[ "$n" = "1" ] || { echo "生存信号が $n 行ある"; exit 1; }

echo "== 26. 稼働状況がその日を「動いていた・記録なし」で返す（FR-54 / design D6）"
cov=$(curl -sf "${AUTH[@]}" "http://$BIND/coverage?from=2026-03-01&to=2026-03-01")
state=$(printf '%s' "$cov" | jq -r '.[]|select(.logical_source=="c01-photo")|.days[0].state')
echo "   → c01-photo 2026-03-01 = $state"
[ "$state" = "alive_no_record" ] || { echo "②のはずが $state"; exit 1; }
# **記録の件数は 0**（稼働が残っているのは生存信号のおかげ）
[ "$(printf '%s' "$cov" | jq -r '.[]|select(.logical_source=="c01-photo")|.days[0].event_count')" = "0" ] \
  || { echo "記録が 0 件でない"; exit 1; }
# 位置は同じ日に記録があるので①
[ "$(printf '%s' "$cov" | jq -r '.[]|select(.logical_source=="c01-location")|.days[0].state')" = "recorded" ] \
  || { echo "位置が①でない"; exit 1; }
# **5 ソースすべてが返る**（画面は 5 本の格子を並べる）
[ "$(printf '%s' "$cov" | jq 'length')" = "5" ] || { echo "5 ソースが返っていない"; exit 1; }

echo "== 27. 達成日数と分母、確定か暫定かが返る（NFR-13 / 第 7 回 Q27）"
ach=$(curl -sf "${AUTH[@]}" "http://$BIND/coverage/achievement")
echo "   → $(printf '%s' "$ach" | jq -c '{verdict, confirmed, not_started: (.not_started|length)}')"
[ "$(printf '%s' "$ach" | jq '.sources|length')" = "5" ] || { echo "5 本ぶん返っていない"; exit 1; }
# 達成日数と分母の両方が返る
printf '%s' "$ach" | jq -e '.sources|all(has("achieved_days") and has("denominator"))' >/dev/null \
  || { echo "達成日数か分母が返っていない"; exit 1; }
# **まだ始まっていないソースがあるので確定日は返らない**（第 7 回 Q27）
[ "$(printf '%s' "$ach" | jq -r '.confirmed')" = "false" ] || { echo "確定になっている"; exit 1; }
[ "$(printf '%s' "$ach" | jq -r '.confirms_on')" = "null" ] || { echo "確定日が返っている"; exit 1; }
printf '%s' "$ach" | jq -e '.not_started|length > 0' >/dev/null \
  || { echo "まだ開始していないソースが示されていない"; exit 1; }

echo "== 28. 生存信号も合言葉を要求する（PERM-10: すべての API 要求）"
for path in "/heartbeat" "/coverage?from=2026-03-01&to=2026-03-01" "/coverage/achievement"; do
  case "$path" in
    /heartbeat) code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$BIND$path" \
                  -H 'content-type: application/json' -d "$beat") ;;
    *)          code=$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND$path") ;;
  esac
  [ "$code" = "401" ] || { echo "$path が 401 のはずが $code"; exit 1; }
done
echo "   → 3 経路とも 401"

echo "縦串 OK（実データ経路と稼働状況まで）"
