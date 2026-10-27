# ST49: 作った部品の版の錠（design D7 / D21（仮））。tools/check-immutable.sh が `.` で読む（psql・fail が使える）。
# 前の版の履歴の版（3 列）と、前の版なしの本表の版の書き換えが拒まれることを、わざと UPDATE を投げて確かめる
# （アプリを通さない経路）。節は本体の各節の戻し手順の後に読まれ、門・錠が古い本体に戻っていることがあるので、
# 全版を当てた状態へ作り直す（tools/check-immutable.d/README.md。作り直すと付与も消えるので当て直す）。
psql -c "DROP SCHEMA core CASCADE;" >/dev/null
for m in "${MIGS[@]}"; do psql < "migrations/$m.sql" >/dev/null; done
./tools/db-roles.sh >/dev/null
docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato_owner -d ashiato < crates/server/src/grants.sql >/dev/null
ST49_EID='49494949-4949-4949-8949-494949494949'
ST49_USER='00000000-0000-0000-0000-000000000000'
echo "== ST49: 版つきの「収集した」記録と、その前の版の履歴を 1 件ずつ置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('st49-version-check','版の錠の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload,
            collector_version, web_version, server_version)
         VALUES ('$ST49_EID','$ST49_USER','st49-version-check','gate-dev','collected',
                 '2026-09-08T02:00:00Z',540,'Asia/Tokyo',
                 1,'st49-version-hash','{\"v\":1}','{\"v\":1}',
                 '1.0+aaaa','1.0+aaaa','1.0+aaaa');" >/dev/null
psql -c "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            collector_version, web_version, server_version)
         VALUES ('$ST49_EID','$ST49_USER','st49-version-check',1,'2026-09-08T02:00:00Z','st49-version-hash',
                 '{\"v\":1}','{\"v\":1}','1.0+aaaa','1.0+aaaa','1.0+aaaa');" >/dev/null

echo "== Scenario: 前の版の履歴の版は書き換えられない"
# 拒んだのが錠の名指し（本文「履歴は追記のみ」）であることまで見る。名指しを外しても後ろの
# 「本文の消去だけが通る」で拒まれ、拒まれたことだけでは名指しの有無が分からない
for st49_col in collector_version web_version server_version; do
  if ST49_ERR=$(psql -c "UPDATE core.event_version SET $st49_col='x+bbbb' WHERE event_id='$ST49_EID';" 2>&1 >/dev/null); then
    echo "  NG 前の版の履歴の版が書き換えられた: $st49_col"; fail=1
  elif [[ "$ST49_ERR" != *"履歴は追記のみ"* ]]; then
    echo "  NG 前の版の履歴の版を拒んだのが錠の名指しでない: $st49_col: $ST49_ERR"; fail=1
  else
    echo "  OK 前の版の履歴の版の書き換えは拒まれた: $st49_col"
  fi
done

echo "== Scenario: 収集した記録の版を、前の版の履歴なしで書き換えると拒まれる"
# 前の版の履歴の行は置いてあるが、**別のまとまり**（txid が違う）なので、この書き換えの履歴にはならない
for st49_col in collector_version web_version server_version; do
  if ST49_ERR=$(psql -c "UPDATE core.event SET $st49_col='x+bbbb' WHERE id='$ST49_EID';" 2>&1 >/dev/null); then
    echo "  NG 前の版の履歴なしで版が書き換えられた: $st49_col"; fail=1
  elif [[ "$ST49_ERR" != *"更新前の版"* ]]; then
    echo "  NG 版の書き換えを拒んだのが門の名指しでない: $st49_col: $ST49_ERR"; fail=1
  else
    echo "  OK 前の版の履歴なしの版の書き換えは拒まれた: $st49_col"
  fi
done
[ "$(psql -c "SELECT collector_version||web_version||server_version FROM core.event WHERE id='$ST49_EID';")" = "1.0+aaaa1.0+aaaa1.0+aaaa" ] \
  || { echo "  NG 拒まれたのに版が変わっている"; fail=1; }
