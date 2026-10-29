# smoke.sh の Story の節（ST23 / tasks 10.1）。滞在を作る → 消す → 確認を読む → 件数を添えて本文ごと消す →
# DB の本文が空・台帳に 1 行・/day に本文ごと消した区間 → 同じ内容を送り直しても重複。
# 使えるもの: post / psql / AUTH / BIND（tools/smoke.sh 本体）。別の利用者・別の日で送り、上の段の件数を動かさない。

# Scenario: 一緒に消えた位置の本文も消える
echo "== 本文ごと消す 1. 位置を送って滞在を作る（ST23）"
PURGE_USER="23232323-0000-4000-8000-000000000023"
purge_items=$(for i in $(seq 0 20); do
  printf '{"id":"%s","user_id":"%s","logical_source":"c01-location","external_id":null,
    "device_id":"smoke-dev","origin":"collected","event_time":"2026-08-22T%02d:%02d:00Z",
    "tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,
    "raw":"{\\"lat\\":35.6812,\\"lon\\":139.7671,\\"acc_m\\":12}",
    "payload":{"lat":35.6812,"lon":139.7671,"acc_m":12}}\n' \
    "$(printf '23000%03d-0000-4000-8000-000000000000' "$i")" "$PURGE_USER" 0 "$i"
done | jq -s -c .)
code=$(post "$purge_items")
[ "$code" = "200" ] || { echo "位置の記録が $code で断られた"; exit 1; }
purge_stay=$(curl -sf "${AUTH[@]}" "http://$BIND/stays?date=2026-08-22&user_id=$PURGE_USER" \
  | jq -r '[.entries[] | select(.kind == "stay")][0].id')
[ -n "$purge_stay" ] && [ "$purge_stay" != "null" ] || { echo "滞在が出ていない"; exit 1; }
echo "   → 滞在 1 件"

echo "== 本文ごと消す 2. 本文ごと消す口は『消した』滞在だけ（消す前は 409 not_erased）"
code=$(curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body -w '%{http_code}' \
  -X POST "http://$BIND/stays/purge" \
  -d "{\"stay_id\":\"$purge_stay\",\"user_id\":\"$PURGE_USER\",\"expected\":{\"stays\":0,\"locations\":0,\"archive_locations\":0,\"archive_items\":0,\"versions\":0}}")
[ "$code" = "409" ] && [ "$(jq -r .reason /tmp/smoke.body)" = "not_erased" ] \
  || { echo "消す前の本文ごと消すが断られていない: $code $(cat /tmp/smoke.body)"; exit 1; }

echo "== 本文ごと消す 3. 『消した』にする"
code=$(curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body -w '%{http_code}' \
  -X POST "http://$BIND/stays/erase" -d "{\"stay_id\":\"$purge_stay\",\"user_id\":\"$PURGE_USER\"}")
[ "$code" = "200" ] || { echo "消すが $code: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-location' AND user_id='$PURGE_USER' AND raw <> '';")" = "21" ] \
  || { echo "『消した』だけで本文が消えている"; exit 1; }

echo "== 本文ごと消す 4. 確認を読む（件数）"
preview=$(curl -sf "${AUTH[@]}" "http://$BIND/stays/purge/preview?stay_id=$purge_stay&user_id=$PURGE_USER")
printf '%s' "$preview" | jq -e '.counts.stays == 1 and .counts.locations == 21 and .purged == false' >/dev/null \
  || { echo "確認の件数が想定と違う: $preview"; exit 1; }
expected=$(printf '%s' "$preview" | jq -c .counts)

echo "== 本文ごと消す 5. 件数を添えて本文ごと消す"
code=$(curl -s "${AUTH[@]}" -H 'content-type: application/json' -o /tmp/smoke.body -w '%{http_code}' \
  -X POST "http://$BIND/stays/purge" -d "{\"stay_id\":\"$purge_stay\",\"user_id\":\"$PURGE_USER\",\"expected\":$expected}")
[ "$code" = "200" ] || { echo "本文ごと消すが $code: $(cat /tmp/smoke.body)"; exit 1; }
jq -e '.operation_id != null' /tmp/smoke.body >/dev/null || { echo "操作の識別子が無い: $(cat /tmp/smoke.body)"; exit 1; }

echo "== 本文ごと消す 6. DB の本文が空・台帳に 1 行・/day に本文ごと消した区間"
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source IN ('c01-location','s01-stay') AND user_id='$PURGE_USER' AND (raw <> '' OR payload <> '{}'::jsonb);")" = "0" ] \
  || { echo "本文が残っている"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-location' AND user_id='$PURGE_USER';")" = "21" ] \
  || { echo "行そのものが消えた（本文だけ消えるはず）"; exit 1; }
ledger=$(psql -c "SELECT count(*) FROM core.erasure_operation WHERE user_id='$PURGE_USER' AND kind='user';" 2>/dev/null) \
  || { echo "台帳の表を読めない"; exit 1; }
[ "$ledger" = "1" ] || { echo "台帳が 1 行でない: $ledger"; exit 1; }
curl -sf "${AUTH[@]}" "http://$BIND/day?date=2026-08-22&user_id=$PURGE_USER" \
  | jq -e '[.entries[] | select(.purged == true)] | length == 1' >/dev/null \
  || { echo "/day に本文ごと消した区間が無い"; exit 1; }

echo "== 本文ごと消す 7. 同じ内容を送り直しても重複（本文は戻らない）"
code=$(post "$purge_items")
[ "$code" = "200" ] || { echo "送り直しが $code"; exit 1; }
[ "$(jq -c '[.[].duplicate] | unique' /tmp/smoke.body)" = "[true]" ] \
  || { echo "送り直しが重複になっていない: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-location' AND user_id='$PURGE_USER' AND raw <> '';")" = "0" ] \
  || { echo "送り直しで本文が戻った"; exit 1; }
echo "OK 本文ごと消す（ST23）"
