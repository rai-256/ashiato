# shellcheck shell=bash disable=SC2034  # fail は本体が読む（源泉は tools/check-immutable.sh）
# ================================================================ ST17: 主観の記録の錠と、2 つの台帳
#
# **取り込み口を通さず psql から直に殴る**（`cargo test` の `st17_lock_` は同じ文を sqlx で撃つが、
# 同じ PC の第三者製プラグインや手作業が打てるのはこの経路）。
# 通すのは **削除の印・感度・その記録の台帳つきの消去**だけ。`s01-feeling` の登録簿の行は移行が入れている。
echo "== ST17: 主観の記録の錠と、2 つの台帳"
# **前の節の状態に依らない。** 利用者の id と本人が書いた行は、この節が自分で置く（本体の `$AU` や `immutable-check` の行は使わない）。
# 主観の版が当たっていなければ、後ろの検査は意味を持たないので NG を出して節を終える。
ST17_USER='c4444444-4444-4444-8444-000000000001'
ST17_AUTHORED='c5555555-5555-4555-8555-000000000001'    # 主観ではない、本人が書いた行
if [ "$(psql -c "SELECT to_regclass('core.feeling_reminder') IS NOT NULL AND EXISTS (SELECT 1 FROM core.source WHERE logical_source='s01-feeling');")" != "t" ]; then
  echo "  NG 主観の版（202610141000_subjective_log）が当たっていない"; fail=1
  return 0
fi
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('st17-authored-check','主観の節の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('$ST17_AUTHORED','$ST17_USER','st17-authored-check','authored',
                 '2026-09-08T03:00:00Z',540,'Asia/Tokyo',1,'st17-authored-hash','{}','{}');" >/dev/null
ST17_FA='c1111111-1111-4111-8111-000000000001'      # 書き換えの拒否と、消去の偽物を全部ここへ当てる
ST17_FB='c2222222-2222-4222-8222-000000000001'      # 削除の印・感度
ST17_FC='c3333333-3333-4333-8333-000000000001'      # 台帳つきの消去が**通る**ことだけを見る

st17_feeling_row_of() {   # $1=id
  printf "INSERT INTO core.event
     (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
      schema_version, sensitivity, content_hash, raw, payload)
   VALUES ('%s','%s','s01-feeling','authored','2026-09-15T02:00:00Z',540,'Asia/Tokyo',1,1,
           'feeling-hash-%s', '{\"valence\":2}','{\"valence\":2}');" "$1" "$ST17_USER" "$1"
}
for id in "$ST17_FA" "$ST17_FB" "$ST17_FC"; do psql -c "$(st17_feeling_row_of "$id")" >/dev/null; done

# 2 つの台帳は追記のみ（種類の登録簿の 2 行は移行が入れている）
psql -c "INSERT INTO core.feeling_reminder (user_id, at) VALUES ('$ST17_USER','20:30');" >/dev/null
for stmt in "UPDATE core.feeling_reminder SET at='07:00';" "DELETE FROM core.feeling_reminder;" \
            "TRUNCATE core.feeling_reminder;" \
            "UPDATE core.feeling_target_kind SET label='x';" "DELETE FROM core.feeling_target_kind;" \
            "TRUNCATE core.feeling_target_kind;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 主観の台帳が変えられた: $stmt"; fail=1
  fi
done
if [ "$(psql -c "SELECT count(*) FROM core.feeling_target_kind WHERE kind IN ('day','stay');")" = "2" ] \
   && [ "$(psql -c "SELECT count(*) FROM core.feeling_reminder WHERE user_id='$ST17_USER' AND at='20:30';")" = "1" ]; then
  echo "  OK feeling 通知の時刻の台帳と紐づけ先の種類の登録簿は追記のみ"
else
  echo "  NG 主観の台帳の行が変わっている"; fail=1
fi

# Scenario: 主観の値を書き換える文は拒まれる
if psql -c "UPDATE core.event SET payload = jsonb_set(payload,'{valence}','-2') WHERE id='$ST17_FA';" \
     >/dev/null 2>&1; then
  echo "  NG 主観の値が書き換えられた（FR-48 違反）"; fail=1
else
  echo "  OK feeling 主観の値の書き換えは拒まれた"
fi
[ "$(psql -c "SELECT payload->>'valence' FROM core.event WHERE id='$ST17_FA';")" = "2" ] \
  || { echo "  NG 拒まれたのに値が変わっている"; fail=1; }

# Scenario: 主観の紐づけ先は書き換えられない
if psql -c "UPDATE core.event SET raw='{\"target\":{\"kind\":\"day\",\"id\":\"2026-09-16\"}}',
                                  payload='{\"target\":{\"kind\":\"day\",\"id\":\"2026-09-16\"}}'
             WHERE id='$ST17_FA';" >/dev/null 2>&1; then
  echo "  NG 主観の紐づけ先が書き換えられた"; fail=1
else
  echo "  OK feeling 主観の紐づけ先は書き換えられない"
fi

st17_feeling_update_rejected() {   # $1=SET 句 $2=OK の行に出す名前
  if psql -c "UPDATE core.event SET $1 WHERE id='$ST17_FA';" >/dev/null 2>&1; then
    echo "  NG 主観の記録の列が書き換えられた: $1"; fail=1
  else
    echo "  OK feeling 主観の記録の$2は書き換えられない"
  fi
}

# Scenario: 主観の出来事の時刻は書き換えられない
st17_feeling_update_rejected "event_time='2000-01-01T00:00:00Z'" "出来事の時刻"
st17_feeling_update_rejected "user_id='11111111-1111-4111-8111-111111111111'" "利用者"
st17_feeling_update_rejected "external_ref='forged-ref'" "外部識別子（external_ref）"
st17_feeling_update_rejected "logical_source='st17-authored-check'" "論理ソース（外への付け替え）"

# Scenario: 主観の行は削除できない
if psql -c "DELETE FROM core.event WHERE id='$ST17_FA';" >/dev/null 2>&1; then
  echo "  NG 主観の記録を行ごと消せた（FR-48 違反）"; fail=1
else
  echo "  OK feeling 主観の記録は行ごと消せない"
fi
[ "$(psql -c "SELECT count(*) FROM core.event WHERE id='$ST17_FA';")" = "1" ] \
  || { echo "  NG 主観の記録が消えている"; fail=1; }

# Scenario: 他の記録を主観へ付け替えられない
if psql -c "UPDATE core.event SET logical_source='s01-feeling'
             WHERE id='$ST17_AUTHORED';" >/dev/null 2>&1; then
  echo "  NG 他の「本人が書いた」記録を主観の記録へ付け替えられた"; fail=1
else
  echo "  OK feeling 他の記録を主観へ付け替えられない"
fi

# Scenario: 主観に削除の印を付けられる
if psql -c "UPDATE core.event SET deleted_at=now(), deleted_by='check' WHERE id='$ST17_FB';" \
     >/dev/null 2>&1; then
  echo "  OK feeling 主観に削除の印を付けられる"
else
  echo "  NG 主観の削除の印まで止めている（FR-50）"; fail=1
fi
psql -c "UPDATE core.event SET deleted_at=NULL, deleted_by=NULL WHERE id='$ST17_FB';" >/dev/null

# Scenario: 主観の感度を変えられる
if psql -c "UPDATE core.event SET sensitivity=3 WHERE id='$ST17_FB';" >/dev/null 2>&1; then
  echo "  OK feeling 主観の感度を変えられる"
else
  echo "  NG 主観の感度まで止めている（PERM-2）"; fail=1
fi

st17_feeling_ledger() {   # $1=台帳に書く記録の id
  printf "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
          VALUES ('%s','%s','s01-feeling','event','check');" "$1" "$ST17_USER"
}

# Scenario: 台帳の無い主観の消去は拒まれる
if psql -c "UPDATE core.event SET raw='', payload='{}' WHERE id='$ST17_FA';" >/dev/null 2>&1; then
  echo "  NG 台帳を書かない消去が通った"; fail=1
else
  echo "  OK feeling 台帳の無い主観の消去は拒まれた"
fi
[ "$(psql -c "SELECT raw <> '' FROM core.event WHERE id='$ST17_FA';")" = "t" ] \
  || { echo "  NG 拒まれたのに原文が消えている"; fail=1; }

# Scenario: 別の記録の台帳の行では主観の消去は通らない
if psql -c "$(st17_feeling_ledger '77777777-7777-4777-8777-777777777778')
            UPDATE core.event SET raw='', payload='{}' WHERE id='$ST17_FA';" >/dev/null 2>&1; then
  echo "  NG 別の記録の台帳 1 行で主観の記録が消去できた（門が「その記録の」を見ていない）"; fail=1
else
  echo "  OK feeling 別の記録の台帳の行では主観の消去は通らない"
fi
[ "$(psql -c "SELECT raw <> '' FROM core.event WHERE id='$ST17_FA';")" = "t" ] \
  || { echo "  NG 別の記録の台帳で原文が消えた"; fail=1; }

# Scenario: 台帳の行があっても消去の形でない主観の書き換えは拒まれる
if psql -c "$(st17_feeling_ledger "$ST17_FA")
            UPDATE core.event SET raw='', payload='{\"valence\":-2}' WHERE id='$ST17_FA';" \
     >/dev/null 2>&1; then
  echo "  NG 消去の顔で解析済みを差し替えられた"; fail=1
else
  echo "  OK feeling 台帳の行があっても、消去の形でない書き換えは拒まれた"
fi

# Scenario: 台帳のある主観の消去は通る
if psql -c "$(st17_feeling_ledger "$ST17_FC")
            UPDATE core.event SET raw='', payload='{}' WHERE id='$ST17_FC';" >/dev/null 2>&1; then
  echo "  OK feeling 台帳のある主観の消去は通る"
else
  echo "  NG 主観の原文が消せない（FR-51）"; fail=1
fi
[ "$(psql -c "SELECT raw = '' AND payload = '{}'::jsonb FROM core.event WHERE id='$ST17_FC';")" = "t" ] \
  || { echo "  NG 通ったのに原文が残っている"; fail=1; }
[ "$(psql -c "SELECT content_hash FROM core.event WHERE id='$ST17_FC';")" = "feeling-hash-$ST17_FC" ] \
  || { echo "  NG 消去で内容の鍵が動いた"; fail=1; }

# Scenario: 主観以外の本人が書いた記録は従来どおりの規則で扱われる
# **主観の行がある DB でも、本人が書いた記録（`st17-authored-check`）は従来どおり書き換えられる**
if psql -c "UPDATE core.event SET payload = '{\"edited\":true}'
            WHERE logical_source = 'st17-authored-check' AND origin = 'authored';" >/dev/null 2>&1; then
  echo "  OK feeling 主観の行があっても、主張・場所・主観以外の本人が書いた記録は書き換えられる"
else
  echo "  NG 主観の行があると本人が書いた記録の書き換えが通らない"; fail=1
fi

# --- 戻し手順（D16）。**主観の記録か通知の時刻が残っていれば、2 表も登録簿の行も残す**
psql < migrations/202610141000_subjective_log.down.sql >/dev/null 2>&1 \
  || { echo "  NG 202610141000_subjective_log.down.sql が当たらない"; fail=1; }
if [ "$(psql -c "SELECT count(*) FROM core.feeling_reminder WHERE user_id='$ST17_USER';")" = "1" ] \
   && [ "$(psql -c "SELECT count(*) FROM core.feeling_target_kind WHERE kind IN ('day','stay');")" = "2" ] \
   && [ "$(psql -c "SELECT count(*) FROM core.source WHERE logical_source='s01-feeling';")" = "1" ] \
   && [ "$(psql -c "SELECT count(*) FROM core.event WHERE id IN ('$ST17_FA','$ST17_FB','$ST17_FC');")" = "3" ]; then
  echo "  OK feeling 主観が残っていれば、戻しても台帳と登録簿の行と主観の記録が残る"
else
  echo "  NG 戻しで主観の行か台帳か登録簿の行が減った"; fail=1
fi
psql < migrations/202610141000_subjective_log.sql >/dev/null 2>&1 \
  || { echo "  NG 戻した後に当て直せない"; fail=1; }
if psql -c "UPDATE core.event SET payload='{\"forged\":1}' WHERE id='$ST17_FA';" >/dev/null 2>&1; then
  echo "  NG 戻して進めた後に主観の記録の錠が消えている"; fail=1
else
  echo "  OK feeling 戻して進めても主観の記録の錠は効いている"
fi
