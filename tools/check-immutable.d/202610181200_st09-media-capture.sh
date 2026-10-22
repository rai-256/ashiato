# ST09: 原本の受け取り・原本の欠け・文字抽出の表は、アプリの役割からも書き換えも削除もできない（design D15 / D17）。
# 管理者の接続ではなく、アプリの役割（ashiato_app）で 1 本ずつ撃つ。拒まれたときだけ OK（通ったら NG）。
# 呼び出し元（tools/check-immutable.sh）が app_psql と grants の付与を済ませている。
echo "== ST09: メディアの 3 表の錠（アプリの役割）"
st09_app() { docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato_app -d ashiato "$@"; }
st09_uid='00000000-0000-0000-0000-000000000009'
st09_ev='99999999-9999-4999-8999-000000000009'
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('c01-photo','端末に保存された写真',21600,'record') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event (id,user_id,logical_source,origin,event_time,tz_offset_min,tz_id,schema_version,content_hash,raw,payload)
         VALUES ('$st09_ev','$st09_uid','c01-photo','collected','2026-10-18T00:00:00Z',540,'Asia/Tokyo',1,'st09-immutable','{}','{}');" >/dev/null
st09_app -c "INSERT INTO core.media_original (user_id, sha256, logical_source, size_bytes)
             VALUES ('$st09_uid','st09-sha','c01-photo',10);" >/dev/null
st09_app -c "INSERT INTO core.media_missing (id,user_id,device_id,logical_source,sha256,reason,noticed_at)
             VALUES ('$st09_ev','$st09_uid','phone-01','c01-photo','st09-sha','vanished',now());" >/dev/null
st09_app -c "INSERT INTO core.text_extraction (id,user_id,event_id,engine,engine_version,extracted_at,text)
             VALUES ('$st09_ev','$st09_uid','$st09_ev','ocr','1.0',now(),'テスト');" >/dev/null

echo "Scenario: 原本の欠けは書き換えられない"
for stmt in "UPDATE core.media_missing SET reason = 'changed' WHERE id = '$st09_ev'" \
            "DELETE FROM core.media_missing WHERE id = '$st09_ev'" \
            "UPDATE core.media_original SET size_bytes = 11 WHERE user_id = '$st09_uid'" \
            "DELETE FROM core.media_original WHERE user_id = '$st09_uid'"; do
  if st09_app -c "$stmt" >/dev/null 2>&1; then
    echo "  NG アプリの役割で変えられた: $stmt"; fail=1
  else
    echo "  OK ${stmt%% WHERE*} を拒んだ"
  fi
done
[ "$(psql -c "SELECT reason FROM core.media_missing WHERE id='$st09_ev';")" = "vanished" ] \
  || { echo "  NG 原本の欠けの行が変わっている"; fail=1; }

echo "Scenario: 文字抽出の行は書き換えられない"
for stmt in "UPDATE core.text_extraction SET engine_version = '2.0' WHERE id = '$st09_ev'" \
            "DELETE FROM core.text_extraction WHERE id = '$st09_ev'"; do
  if st09_app -c "$stmt" >/dev/null 2>&1; then
    echo "  NG アプリの役割で変えられた: $stmt"; fail=1
  else
    echo "  OK ${stmt%% WHERE*} を拒んだ"
  fi
done
[ "$(psql -c "SELECT engine_version FROM core.text_extraction WHERE id='$st09_ev';")" = "1.0" ] \
  || { echo "  NG 文字抽出の行が変わっている"; fail=1; }
# TRUNCATE は権限が無い（3 表とも）。管理者でも文トリガが拒む
for t in media_original media_missing text_extraction; do
  if st09_app -c "TRUNCATE core.$t" >/dev/null 2>&1 || psql -c "TRUNCATE core.$t" >/dev/null 2>&1; then
    echo "  NG $t を切り詰められた"; fail=1
  else
    echo "  OK $t の TRUNCATE を拒んだ"
  fi
done
