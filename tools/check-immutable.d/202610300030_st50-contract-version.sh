# ST50: 契約の版の錠（design D20）。tools/check-immutable.sh が `.` で読む（psql・fail が使える）。
# 前の版の契約の版と、収集した記録の契約の版だけの書き換えが拒まれることを、わざと UPDATE を投げて確かめる（アプリを通さない経路）。
# 節は本体の各節の戻し手順の後に読まれ、門・錠が古い本体に戻っていることがあるので、全版を当てた状態へ作り直す（tools/check-immutable.d/README.md）。
psql -c "DROP SCHEMA core CASCADE;" >/dev/null
for m in "${MIGS[@]}"; do psql < "migrations/$m.sql" >/dev/null; done
# 作り直しで付与（grants.sql）も消えたので、読む口の前の準備と同じに当て直す（後ろの節が `app_psql` で撃てるように）
./tools/db-roles.sh >/dev/null
docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato_owner -d ashiato < crates/server/src/grants.sql >/dev/null
ST50_EID='50505050-5050-4050-8050-505050505050'
ST50_USER='00000000-0000-0000-0000-000000000000'
st50_version() { psql -c "SELECT coalesce(contract_version::text, 'null') FROM core.event WHERE id='$ST50_EID';"; }
st50_prev_version() { psql -c "SELECT coalesce(contract_version::text, 'null') FROM core.event_version WHERE event_id='$ST50_EID';"; }
echo "== ST50: 契約の版が 1 の「収集した」記録を 1 件と、その前の版を置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('st50-version-check','契約の版の錠の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, contract_version, content_hash, raw, payload)
         VALUES ('$ST50_EID','$ST50_USER','st50-version-check','gate-dev','collected',
                 '2026-09-08T02:00:00Z',540,'Asia/Tokyo',1,1,'st50-hash','{\"v\":1}','{\"v\":1}');" >/dev/null
psql -c "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload, contract_version)
         VALUES ('$ST50_EID','$ST50_USER','st50-version-check',1,'2026-09-08T02:00:00Z','st50-hash',
                 '{\"v\":1}','{\"v\":1}',1);" >/dev/null

echo "== Scenario: 前の版の契約の版は書き換えられない"
# 拒んだのが錠の名指し（本文「履歴は追記のみ」）であることまで見る。名指しを外しても、後ろの条件で別の理由で拒まれるため
if ST50_ERR=$(psql -c "UPDATE core.event_version SET contract_version=2 WHERE event_id='$ST50_EID';" 2>&1 >/dev/null); then
  echo "  NG 前の版の契約の版が書き換えられた"; fail=1
elif [[ "$ST50_ERR" != *"履歴は追記のみ"* ]]; then
  echo "  NG 前の版の契約の版を拒んだのが錠の名指しでない: $ST50_ERR"; fail=1
else
  echo "  OK 前の版の契約の版の書き換えは拒まれた"
fi
[ "$(st50_prev_version)" = "1" ] || { echo "  NG 拒まれたのに前の版の契約の版が変わっている: $(st50_prev_version)"; fail=1; }

echo "== Scenario: 収集した記録の契約の版だけを前の版なしで書き換えると拒まれる"
if psql -c "UPDATE core.event SET contract_version=2 WHERE id='$ST50_EID';" >/dev/null 2>&1; then
  echo "  NG 契約の版だけを前の版なしで変えられた"; fail=1
else
  echo "  OK 契約の版だけを前の版なしで変えることは拒まれた"
fi
[ "$(st50_version)" = "1" ] || { echo "  NG 拒まれたのに契約の版が変わっている: $(st50_version)"; fail=1; }

echo "== ST50: 埋め戻しで外した 2 つの錠が張られている"
ST50_ARMED=$(psql -c "SELECT count(*) FROM pg_trigger
                       WHERE tgname IN ('event_version_immutable', 'event_requires_version') AND tgenabled = 'O';")
if [ "$ST50_ARMED" = "2" ]; then
  echo "  OK 前の版の錠と本表の門は有効（tgenabled = 'O'）"
else
  echo "  NG 前の版の錠か本表の門が有効でない（有効な数: $ST50_ARMED）"; fail=1
fi
