# ST11: 健康データの権限の記録（c01-health-permission）の縦串。tools/smoke.sh の末尾が `.` で読む
# 端末の側は計測テスト（HealthPermissionGrantInstrumentedTest）が見る。形は design D2 と同じ
# （端末が作る形が D2 と同じことは HealthPermissionRecordTest の形の試験と docs/collector-contract.md が固定する）。
# ここは本物のサーバに、端末が送る形の記録を送り、D-01 の行を見る。

HEALTH_USER="00000000-0000-0000-0000-000000000000"
HEALTH_TIME="2026-10-24T01:00:00.123Z"
health_payload='{"kind":"health-permission","trigger":"request_result","device_time":"2026-10-24T01:00:00.123Z","elapsed_ms":123456789,"boot_count":42,"availability":"available","features":{"history":true,"background":true},"granted":{"sleep":true,"heart_rate":true,"steps":true,"exercise":true,"history":true,"background":true},"auto_requested":true}'
health_record() {
  printf '[{"id":"11000001-0000-4000-8000-000000000001","user_id":"%s","logical_source":"c01-health-permission","external_id":null,"device_id":"health-smoke","origin":"collected","event_time":"%s","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,"raw":%s,"payload":%s}]' \
    "$HEALTH_USER" "$HEALTH_TIME" "$(rawstr "$health_payload")" "$health_payload"
}

# Scenario: 許すと許した状態と観た時刻の記録が S-01 に格納される
echo "== ST11. 健康データの権限の記録（4 種類・履歴・背景を許した形）を送り、D-01 の 1 行を見る。もう 1 度送っても増えない"
code=$(post "$(health_record)")
[ "$code" = "200" ] || { echo "健康データの権限の記録が $code: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(jq -r '.[0].accepted' /tmp/smoke.body)" = "true" ] || { echo "健康データの権限の記録が受理されない: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(jq -r '.[0].duplicate' /tmp/smoke.body)" = "false" ] || { echo "1 度目が重複と判定された"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-health-permission';")" = "1" ] \
  || { echo "健康データの権限の記録が D-01 に 1 行入っていない"; exit 1; }
[ "$(psql -c "SELECT payload->'granted' = '{\"sleep\":true,\"heart_rate\":true,\"steps\":true,\"exercise\":true,\"history\":true,\"background\":true}'::jsonb FROM core.event WHERE logical_source='c01-health-permission';")" = "t" ] \
  || { echo "4 種類・履歴・背景の許可が全部 true で格納されていない"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-health-permission' AND event_time = '$HEALTH_TIME'::timestamptz;")" = "1" ] \
  || { echo "event_time が送った device_time と違う"; exit 1; }
code=$(post "$(health_record)")
[ "$code" = "200" ] || { echo "健康データの権限の記録の再送が $code"; exit 1; }
[ "$(jq -r '.[0].duplicate' /tmp/smoke.body)" = "true" ] || { echo "再送が duplicate: true でない: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-health-permission';")" = "1" ] \
  || { echo "再送で行が増えた"; exit 1; }
