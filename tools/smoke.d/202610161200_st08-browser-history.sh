# ST08: ブラウザ履歴（c02-browser-history）の縦串。tools/smoke.sh の末尾が `.` で読む

echo "== ST08. 同じ履歴を2回取り込んでも行を増やさない"
HISTORY_DB=$(mktemp)
rm -f "$HISTORY_DB"
cargo run -q -p ashiato-collector-windows --example browser_history_smoke -- "$HISTORY_DB"
[ -s "$HISTORY_DB" ] || { echo "履歴 DB の台本が作れなかった"; exit 1; }
# 本文は design D4 の形（収集側の `visit_payload_shape_is_pinned` と同じ欄の並び。プロファイルは `profile_dir`）
history_raw='{"kind":"visit","at":"2026-09-08T02:00:00.000000Z","browser":"chrome","family":"chromium","profile_dir":"Default","visit_id":1,"visit_time_raw":13402627200000000,"url":"https://example.test/yesterday","title":"前日のページ","transition":1,"transition_core":"typed","tz_basis":"collected-at"}'
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
vanished_raw='{"kind":"vanished","at":"2026-09-10T02:00:00.000000Z","browser":"chrome","family":"chromium","profile_dir":"Default","tz_basis":"collected-at","vanished":[{"external_id":"v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","age_days":2,"foreign":false,"table_recreated":false,"profile_gone":false}]}'
vanished_body="[{\"id\":\"f2222222-2222-4222-8222-222222222222\",\"user_id\":\"00000000-0000-0000-0000-000000000000\",\"logical_source\":\"c02-browser-history\",\"external_id\":\"v1:vanished:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"device_id\":\"history-smoke\",\"origin\":\"collected\",\"event_time\":\"2026-09-10T02:00:00.000000Z\",\"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,\"source_updated_at\":\"2026-09-10T02:00:00.000Z\",\"raw\":$(rawstr "$vanished_raw"),\"payload\":$vanished_raw}]"
code=$(post "$vanished_body"); [ "$code" = "200" ] || { echo "消えた記録の送信が $code"; exit 1; }
history_after_raw=$(psql -c "SELECT raw FROM core.event
  WHERE logical_source='c02-browser-history'
    AND external_id='v1:visit:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';")
[ "$history_before_raw" = "$history_after_raw" ] || { echo "消えた記録で元の訪問の原文が変わった"; exit 1; }
vanished_rows=$(psql -c "SELECT count(*) FROM core.event
  WHERE logical_source='c02-browser-history' AND payload->>'kind'='vanished';")
[ "$vanished_rows" = "1" ] || { echo "消えた記録が 1 件入っていない: $vanished_rows"; exit 1; }

# Scenario: ブラウザ履歴の記録も既定の感度で格納される
echo "== ST08. 履歴の記録は既定の感度で格納する"
history_sensitivities=$(psql -c "SELECT DISTINCT sensitivity FROM core.event
  WHERE logical_source='c02-browser-history' ORDER BY sensitivity;")
[ "$history_sensitivities" = "1" ] || { echo "履歴の感度が既定ではない: $history_sensitivities"; exit 1; }

echo "== ST08. 前日の履歴を手で組んだ記録で格納する"
history_yesterday=$(psql -c "SELECT payload->>'url'||' '||event_time::text FROM core.event
  WHERE logical_source='c02-browser-history'
    AND payload->>'url'='https://example.test/yesterday';")
case "$history_yesterday" in
  'https://example.test/yesterday 2026-09-08 02:00:00+00') : ;;
  *) echo "前日の URL と訪問時刻が格納されていない: $history_yesterday"; exit 1 ;;
esac

# deep.md 第 4 回 Q8: URL の行を失った訪問は URL と題名を省いた visit として送る。取り込み口はその形を変えずに受ける
echo "== ST08. URL の無い訪問を取り込み口がそのまま格納する"
orphan_raw='{"kind":"visit","at":"2026-09-08T03:00:00.000000Z","browser":"chrome","family":"chromium","profile_dir":"Default","visit_id":2,"visit_time_raw":13402630800000000,"transition":1,"transition_core":"typed","visit_duration_us":1500000,"tz_basis":"collected-at"}'
orphan_body="[{\"id\":\"f3333333-3333-4333-8333-333333333333\",\"user_id\":\"00000000-0000-0000-0000-000000000000\",\"logical_source\":\"c02-browser-history\",\"external_id\":\"v1:visit:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\",\"device_id\":\"history-smoke\",\"origin\":\"collected\",\"event_time\":\"2026-09-08T03:00:00.000000Z\",\"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,\"source_updated_at\":\"2026-09-09T02:00:00.000Z\",\"raw\":$(rawstr "$orphan_raw"),\"payload\":$orphan_raw}]"
code=$(post "$orphan_body"); [ "$code" = "200" ] || { echo "URL の無い訪問の送信が $code"; exit 1; }
orphan=$(psql -c "SELECT coalesce(payload->>'url','-')||' '||coalesce(payload->>'title','-')||' '||(payload->>'visit_duration_us')||' '||event_time::text
  FROM core.event WHERE logical_source='c02-browser-history'
    AND external_id='v1:visit:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';")
[ "$orphan" = "- - 1500000 2026-09-08 03:00:00+00" ] || { echo "URL の無い訪問が格納されていない: $orphan"; exit 1; }

# Scenario: 2 回続けて取得しても行が増えない
echo "== ST08. 同じ履歴 DB を収集側で 2 回取得しても、取り込み口まで通して行が増えない"
FETCH_DB=$(mktemp); rm -f "$FETCH_DB"
cargo build -q -p ashiato-collector-windows --example browser_history_smoke
HSMOKE=./target/debug/examples/browser_history_smoke
"$HSMOKE" make "$FETCH_DB" https://example.test/collected 収集した訪問
count_history() { psql -c "SELECT count(*) FROM core.event WHERE logical_source='c02-browser-history';"; }
h0=$(count_history)
STATE1=$(mktemp -d)
"$HSMOKE" fetch "$FETCH_DB" "$STATE1" "http://$BIND" "$API_TOKEN"
h1=$(count_history)
[ "$h1" = "$((h0 + 1))" ] || { echo "取得した訪問が 1 行だけ増えていない: $h0 -> $h1"; exit 1; }
# 置き場を作り直して（再導入と同じ）同じ履歴 DB を取得し直す。行は増えない
STATE2=$(mktemp -d)
"$HSMOKE" fetch "$FETCH_DB" "$STATE2" "http://$BIND" "$API_TOKEN"
h2=$(count_history)
[ "$h1" = "$h2" ] || { echo "2 回目の取得で行が増えた: $h1 -> $h2"; exit 1; }

# Scenario: 取り込み口が止まっている間に取得した履歴が後から届く
echo "== ST08. 取り込み口を止めて取得し、戻した後に送って格納されている"
kill "$SRV"; wait "$SRV" 2>/dev/null || true
DOWN_DB=$(mktemp); rm -f "$DOWN_DB"
"$HSMOKE" make "$DOWN_DB" https://example.test/while-down 止まっている間の訪問
STATE3=$(mktemp -d)
"$HSMOKE" fetch "$DOWN_DB" "$STATE3" "http://$BIND" "$API_TOKEN"
env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL ./target/debug/ashiato-server & SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$BIND/healthz" >/dev/null
[ "$(psql -c "SELECT count(*) FROM core.event WHERE payload->>'url'='https://example.test/while-down';")" = "0" ] \
  || { echo "取り込み口が止まっている間に格納されている"; exit 1; }
"$HSMOKE" send "$STATE3" "http://$BIND" "$API_TOKEN"
down_rows=$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c02-browser-history' AND payload->>'url'='https://example.test/while-down';")
[ "$down_rows" = "1" ] || { echo "戻した後に届いていない: $down_rows"; exit 1; }
rm -rf "$FETCH_DB" "$DOWN_DB" "$STATE1" "$STATE2" "$STATE3"

# Scenario: 前日に見たページが翌日の取得で入っている
echo "== ST08. 取得契機を 1 日進めて取得し、前日の訪問を取り込み口まで通して psql で行を見る"
NEXT_DB=$(mktemp); rm -f "$NEXT_DB"
"$HSMOKE" make "$NEXT_DB" https://example.test/next-day 前日に見たページ
STATE4=$(mktemp -d)
[ "$(psql -c "SELECT count(*) FROM core.event WHERE payload->>'url'='https://example.test/next-day';")" = "0" ] \
  || { echo "取得の前に格納されている（検査が空振り）"; exit 1; }
"$HSMOKE" next-day "$NEXT_DB" "$STATE4" "http://$BIND" "$API_TOKEN"
next_day_row=$(psql -c "SELECT payload->>'url'||' '||event_time::text FROM core.event
  WHERE logical_source='c02-browser-history' AND payload->>'url'='https://example.test/next-day';")
[ "$next_day_row" = 'https://example.test/next-day 2025-09-18 00:00:00+00' ] \
  || { echo "翌日の取得で前日の URL と訪問時刻が格納されていない: $next_day_row"; exit 1; }
rm -rf "$NEXT_DB" "$STATE4"

rm -f "$HISTORY_DB"
