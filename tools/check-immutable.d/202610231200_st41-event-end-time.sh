# ST41: 終わりの時刻の錠（design D5）。tools/check-immutable.sh が `.` で読む（psql・fail が使える）。
# 3 つが拒まれることを、わざと UPDATE を投げて確かめる（アプリを通さない経路）。
# 節は本体の各節の戻し手順（`.down.sql` → その版の当て直し）の後に読まれ、門・錠が古い本体に戻っていることがある。
# 終わりの錠を見るので、全版を当てた状態へ作り直す（tools/check-immutable.d/README.md）。
psql -c "DROP SCHEMA core CASCADE;" >/dev/null
for m in "${MIGS[@]}"; do psql < "migrations/$m.sql" >/dev/null; done
EID='41414141-4141-4141-8141-414141414141'
EUSER='00000000-0000-0000-0000-000000000000'
echo "== ST41: 終わりつきの「収集した」記録を 1 件置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('st41-end-check','終わりの錠の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            end_time, end_tz_offset_min, end_tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('$EID','$EUSER','st41-end-check','gate-dev','collected',
                 '2026-09-08T02:00:00Z',540,'Asia/Tokyo',
                 '2026-09-08T03:00:00Z',540,'Asia/Tokyo',
                 1,'st41-end-hash','{\"v\":1}','{\"v\":1}');" >/dev/null
st41_end() { psql -c "SELECT to_char(end_time AT TIME ZONE 'UTC','HH24:MI') FROM core.event WHERE id='$EID';"; }

echo "== Scenario: 収集した記録の終わりの時刻だけを履歴なしで変えると拒まれる"
if psql -c "UPDATE core.event SET end_time='2026-09-08T05:00:00Z' WHERE id='$EID';" >/dev/null 2>&1; then
  echo "  NG 終わりの時刻だけを履歴なしで変えられた"; fail=1
else
  echo "  OK 終わりの時刻だけを履歴なしで変えることは拒まれた"
fi
[ "$(st41_end)" = "03:00" ] || { echo "  NG 拒まれたのに終わりの時刻が変わっている: $(st41_end)"; fail=1; }

echo "== Scenario: 終わりだけが違う履歴を書いても、終わりの書き換えは通らない"
# 原文・内容・始まり・鍵は更新前と同じで、終わりだけが更新前と違う履歴
if psql -c "INSERT INTO core.event_version
              (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
               end_time, end_tz_offset_min, end_tz_id)
            VALUES ('$EID','$EUSER','st41-end-check',1,'2026-09-08T02:00:00Z','st41-end-hash',
                    '{\"v\":1}','{\"v\":1}','2026-09-08T05:00:00Z',540,'Asia/Tokyo');
            UPDATE core.event SET end_time='2026-09-08T05:00:00Z' WHERE id='$EID';" >/dev/null 2>&1; then
  echo "  NG 終わりだけ違う履歴の 1 行で終わりの書き換えが通った"; fail=1
else
  echo "  OK 終わりだけ違う履歴では終わりの書き換えは通らない"
fi
[ "$(st41_end)" = "03:00" ] || { echo "  NG 拒まれたのに終わりの時刻が変わっている: $(st41_end)"; fail=1; }

echo "== Scenario: 履歴の前の版の終わりは書き換えられない"
# 更新前の版（終わりも同じ）を正しく書いて、履歴の行を作る
psql -c "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            end_time, end_tz_offset_min, end_tz_id)
         VALUES ('$EID','$EUSER','st41-end-check',1,'2026-09-08T02:00:00Z','st41-end-hash',
                 '{\"v\":1}','{\"v\":1}','2026-09-08T03:00:00Z',540,'Asia/Tokyo');" >/dev/null
for set in "end_time='2026-09-08T05:00:00Z'" "end_tz_offset_min=0" "end_tz_id='UTC'"; do
  if psql -c "UPDATE core.event_version SET $set WHERE event_id='$EID';" >/dev/null 2>&1; then
    echo "  NG 前の版の終わりが書き換えられた: $set"; fail=1
  else
    echo "  OK 前の版の終わりの書き換えは拒まれた: $set"
  fi
done
