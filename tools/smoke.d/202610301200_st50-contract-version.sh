# ST50: 契約の版。本物の起動・資格情報・DB を通して、確かめる口・426・今の形の 1 件・知らない欄の 1 件を見る。tools/smoke.sh の末尾が `.` で読む
# Scenario の印は crates/server/src/contract_version_tests.rs が持つ。ここは行を psql で見る確かめ。

st50_events() { psql -c "SELECT count(*) FROM core.event WHERE logical_source = 'smoke';"; }
st50_body() {  # $1 = id
  printf '{"id":"%s","user_id":"00000000-0000-0000-0000-000000000000","logical_source":"smoke","external_id":null,"device_id":"smoke-dev","origin":"collected","event_time":"2026-09-09T02:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,"raw":"{\\"st50\\":1}","payload":{"st50":1}}' "$1"
}

echo "== ST50. GET /contract は今の版を答える"
st50_get=$(curl -s "${AUTH[@]}" "http://$BIND/contract")
[ "$(printf '%s' "$st50_get" | jq -c '.')" = '{"contract_version":1}' ] || { echo "GET /contract が違う: $st50_get"; exit 1; }
echo "OK /contract"

echo "== ST50. 知らない版の見出しの /ingest は 426 で、行が増えない"
st50_before=$(st50_events)
st50_426=$(curl -s "${AUTH[@]}" -o /tmp/st50-426.$$ -w '%{http_code}' -X POST "http://$BIND/ingest" \
  -H 'content-type: application/json' -H 'Ashiato-Contract-Version: 2' \
  -d "$(st50_body 50000000-0000-4000-8000-000000000001)")
st50_426_body=$(cat /tmp/st50-426.$$); rm -f /tmp/st50-426.$$
[ "$st50_426" = "426" ] || { echo "426 のはずが $st50_426"; exit 1; }
[ "$(printf '%s' "$st50_426_body" | jq -cS '.')" = '{"contract_version":1,"error":"contract_unsupported"}' ] || { echo "426 の本文が違う: $st50_426_body"; exit 1; }
[ "$(st50_events)" = "$st50_before" ] || { echo "426 なのに行が増えた"; exit 1; }
echo "OK 426"

echo "== ST50. 見出しなしの今の形の 1 件は入り、contract_version が 1"
st50_ok=$(curl -s "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' \
  -d "$(st50_body 50000000-0000-4000-8000-000000000002)")
[ "$(printf '%s' "$st50_ok" | jq -r '.[0].accepted')" = "true" ] || { echo "今の形の 1 件が入らない: $st50_ok"; exit 1; }
[ "$(st50_events)" = "$((st50_before + 1))" ] || { echo "行が 1 つ増えていない"; exit 1; }
[ "$(psql -c "SELECT contract_version FROM core.event WHERE id = '50000000-0000-4000-8000-000000000002';")" = "1" ] \
  || { echo "contract_version が 1 でない"; exit 1; }
echo "OK 今の形"

echo "== ST50. 知らない最上位の欄の 1 件は unknown_field で、行が増えない"
st50_unk_body=$(st50_body 50000000-0000-4000-8000-000000000003 | jq -c '. + {mood_score: 3}')
st50_unk=$(curl -s "${AUTH[@]}" -X POST "http://$BIND/ingest" -H 'content-type: application/json' -d "$st50_unk_body")
[ "$(printf '%s' "$st50_unk" | jq -r '.[0].error')" = "unknown_field" ] || { echo "unknown_field でない: $st50_unk"; exit 1; }
[ "$(st50_events)" = "$((st50_before + 1))" ] || { echo "unknown_field なのに行が増えた"; exit 1; }
echo "OK unknown_field"
