# ST08: ブラウザ履歴（c02-browser-history）の縦串。tools/smoke.sh の末尾が `.` で読む

# Scenario: 2 回続けて取得しても行が増えない
echo "== ST08. 同じ履歴を2回取り込んでも行を増やさない"
HISTORY_DB=$(mktemp)
rm -f "$HISTORY_DB"
cargo run -q -p ashiato-collector-windows --example browser_history_smoke -- "$HISTORY_DB"
[ -s "$HISTORY_DB" ] || { echo "履歴 DB の台本が作れなかった"; exit 1; }
history_raw='{"kind":"visit","at":"2026-09-08T02:00:00.000000Z","tz_basis":"collected-at","browser":"chrome","profile":"Default","url":"https://example.test/yesterday","title":"前日のページ"}'
history_body="[{\"id\":\"f1111111-1111-4111-8111-111111111111\",\"user_id\":\"00000000-0000-0000-0000-000000000000\",\"logical_source\":\"c02-browser-history\",\"external_id\":\"v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"device_id\":\"history-smoke\",\"origin\":\"collected\",\"event_time\":\"2026-09-08T02:00:00.000000Z\",\"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,\"source_updated_at\":\"2026-09-09T02:00:00.000Z\",\"raw\":$(rawstr "$history_raw"),\"payload\":$history_raw}]"
code=$(post "$history_body"); [ "$code" = "200" ] || { echo "履歴の初回送信が $code"; exit 1; }
before=$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c02-browser-history';")
code=$(post "$history_body"); [ "$code" = "200" ] || { echo "履歴の再送が $code"; exit 1; }
after=$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c02-browser-history';")
[ "$before" = "$after" ] || { echo "同じ履歴の再送で行が増えた: $before -> $after"; exit 1; }

# Scenario: 消えた訪問の記録は残っている
echo "== ST08. 消えた記録を送っても元の訪問を変えない"
history_before_raw=$(psql -c "SELECT raw FROM core.event
  WHERE logical_source='c02-browser-history'
    AND external_id='v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';")
vanished_raw='{"kind":"vanished","at":"2026-09-10T02:00:00.000000Z","tz_basis":"collected-at","browser":"chrome","profile":"Default","vanished":[{"external_id":"v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","age_days":2,"foreign":false,"table_recreated":false,"profile_gone":false}]}'
vanished_body="[{\"id\":\"f2222222-2222-4222-8222-222222222222\",\"user_id\":\"00000000-0000-0000-0000-000000000000\",\"logical_source\":\"c02-browser-history\",\"external_id\":\"v1:vanished:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"device_id\":\"history-smoke\",\"origin\":\"collected\",\"event_time\":\"2026-09-10T02:00:00.000000Z\",\"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,\"source_updated_at\":\"2026-09-10T02:00:00.000Z\",\"raw\":$(rawstr "$vanished_raw"),\"payload\":$vanished_raw}]"
code=$(post "$vanished_body"); [ "$code" = "200" ] || { echo "消えた記録の送信が $code"; exit 1; }
history_after_raw=$(psql -c "SELECT raw FROM core.event
  WHERE logical_source='c02-browser-history'
    AND external_id='v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';")
[ "$history_before_raw" = "$history_after_raw" ] || { echo "消えた記録で元の訪問の原文が変わった"; exit 1; }

# Scenario: ブラウザ履歴の記録も既定の感度で格納される
echo "== ST08. 履歴の記録は既定の感度で格納する"
history_sensitivities=$(psql -c "SELECT DISTINCT sensitivity FROM core.event
  WHERE logical_source='c02-browser-history' ORDER BY sensitivity;")
[ "$history_sensitivities" = "1" ] || { echo "履歴の感度が既定ではない: $history_sensitivities"; exit 1; }

# Scenario: 前日に見たページが翌日の取得で入っている
echo "== ST08. 前日の履歴を翌日の取得で格納する"
history_yesterday=$(psql -c "SELECT payload->>'url'||' '||event_time::text FROM core.event
  WHERE logical_source='c02-browser-history'
    AND payload->>'url'='https://example.test/yesterday';")
case "$history_yesterday" in
  'https://example.test/yesterday 2026-09-08 02:00:00+00') : ;;
  *) echo "前日の URL と訪問時刻が格納されていない: $history_yesterday"; exit 1 ;;
esac
rm -f "$HISTORY_DB"
