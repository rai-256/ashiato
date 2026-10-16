# smoke.sh の Story の節（tools/smoke.d/ の決まりは README.md）。本体の末尾から移した（2026-10-15）
# ================================================================ 主観（ST17 / tasks 6.2）
# 取り込み口から快–不快だけの主観を 1 件入れて、`GET /feelings`（読み出し）と `GET /feelings/reminder`（通知の判断）で読む。
# 偽データの利用者とは別の利用者で書く（主観の行は DB の錠で消せないので、他の段の数に混ぜない）。
echo "== 45. 主観を 1 件入れて、読み出しと通知の判断で読む"
echo "Scenario: 快–不快だけで記録が 1 件入る"
FEEL_USER="17171717-0000-4000-8000-000000000017"
FEEL_ID="17171717-0000-4000-8000-0000000000f1"
FEEL_DAY="2026-10-05"
FEEL_RAW="{\"feeling\":\"$FEEL_ID\",\"nonce\":\"Zm9vYmFyYmF6cXV4MTIzNDU2\",\"target\":{\"kind\":\"day\",\"date\":\"$FEEL_DAY\"},\"written_at\":\"${FEEL_DAY}T21:00:00+09:00\",\"written_tz\":\"Asia/Tokyo\",\"scales\":[{\"id\":\"valence\",\"version\":1,\"value\":1}],\"note\":null}"
feel_before=$(curl -fsS "${AUTH[@]}" "http://$BIND/feelings/reminder?date=$FEEL_DAY&user_id=$FEEL_USER")
printf '%s' "$feel_before" | jq -e '.recorded == false and .at == "21:00" and (keys | sort) == ["at","recorded"]' >/dev/null \
  || { echo "書く前の通知の判断が合わない: $feel_before"; exit 1; }
feel_res=$(jq -n --arg id "$FEEL_ID" --arg user "$FEEL_USER" --arg raw "$FEEL_RAW" '[{
    id: $id, user_id: $user, logical_source: "s01-feeling", external_id: null, device_id: null, origin: "authored",
    event_time: "2026-10-04T15:00:00Z", tz_offset_min: 540, tz_id: "Asia/Tokyo", schema_version: 1, raw: $raw, payload: {}}]' \
  | curl -fsS "${AUTH[@]}" -H "content-type: application/json" -X POST "http://$BIND/ingest" --data-binary @-)
printf '%s' "$feel_res" | jq -e 'length == 1 and .[0].accepted == true' >/dev/null \
  || { echo "主観が入らなかった: $feel_res"; exit 1; }
feel_read=$(curl -fsS "${AUTH[@]}" "http://$BIND/feelings?from=$FEEL_DAY&to=$FEEL_DAY&user_id=$FEEL_USER")
printf '%s' "$feel_read" | jq -e --arg id "$FEEL_ID" '
  .days[0].status == "recorded" and (.days[0].day.records | length) == 1
  and .days[0].day.latest.before.id == $id and .days[0].day.latest.before.scales[0].value == 1
  and .days[0].day.latest.after == null' >/dev/null \
  || { echo "入れた主観が読み出しに出ていない: $feel_read"; exit 1; }
feel_after=$(curl -fsS "${AUTH[@]}" "http://$BIND/feelings/reminder?date=$FEEL_DAY&user_id=$FEEL_USER")
printf '%s' "$feel_after" | jq -e '.recorded == true and (keys | sort) == ["at","recorded"]' >/dev/null \
  || { echo "書いた後の通知の判断が合わない: $feel_after"; exit 1; }
echo "OK feeling ingest + read + reminder"
