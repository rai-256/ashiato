# 派生の記録（滞在）の本文の消去の門と、消去の操作の台帳が書き換えられないことを、psql から直に確かめる
# （ST23 / design D4 / D7）。取り込み口の外の経路 —— 同じ PC の手作業の psql を止められるのは DB の側だけ。
# `psql -c "a; b;"` は 1 文字列 = 1 トランザクションなので、「同じまとまりに台帳の行がある」を 1 回で撃てる。
# 門は COMMIT の時に落ちる。**拒まれたときだけ OK を出す**（通ってしまったら NG）。

DU="00000000-0000-0000-0000-0000000000d7"
stay_raw() { psql -c "SELECT raw FROM core.event WHERE id = '$1';"; }
put_stay() {
  psql -c "INSERT INTO core.event
             (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
              schema_version, content_hash, raw, payload)
           VALUES ('$1', '$DU', 's01-stay', 'immutable-check', 'derived', '2026-10-01T00:00:00Z',
                   540, 'Asia/Tokyo', 1, 'derived-erasure-$1', '{\"end\":\"2026-10-01T00:20:00Z\"}',
                   '{\"end\":\"2026-10-01T00:20:00Z\"}');" >/dev/null
}
ledger_for() {
  printf "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
          VALUES ('%s', '%s', 's01-stay', 'event', 'immutable-check');" "$1" "$DU"
}
ERASE_SQL() { printf "UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = '%s';" "$1"; }
KEPT='{"end":"2026-10-01T00:20:00Z"}'

# Scenario: 台帳を書かない滞在の本文の消去は拒まれる
S1="d7000000-0000-4000-8000-000000000001"; put_stay "$S1"
if psql -c "$(ERASE_SQL "$S1")" >/dev/null 2>&1; then
  echo "  NG 台帳を書かない滞在の本文の消去が通った"; fail=1
elif [ "$(stay_raw "$S1")" = "$KEPT" ]; then
  echo "  OK 台帳を書かない滞在の本文の消去は拒まれ、本文は残っている"
else
  echo "  NG 拒まれたのに本文が変わっている"; fail=1
fi

# Scenario: 前の版を残した滞在の本文の消去は拒まれる
S2="d7000000-0000-4000-8000-000000000002"; put_stay "$S2"
psql -c "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
         SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload
           FROM core.event WHERE id = '$S2';" >/dev/null
if psql -c "$(ledger_for "$S2") $(ERASE_SQL "$S2")" >/dev/null 2>&1; then
  echo "  NG 前の版を残した滞在の本文の消去が通った"; fail=1
elif [ "$(stay_raw "$S2")" = "$KEPT" ]; then
  echo "  OK 前の版を残した滞在の本文の消去は拒まれ、本文は残っている"
else
  echo "  NG 拒まれたのに本文が変わっている"; fail=1
fi

# Scenario: 解析済みを残して原文だけ空にする書き換えは拒まれる
S3="d7000000-0000-4000-8000-000000000003"; put_stay "$S3"
if psql -c "$(ledger_for "$S3") UPDATE core.event SET raw = '' WHERE id = '$S3';" >/dev/null 2>&1; then
  echo "  NG 解析済みを残して原文だけ空にする書き換えが通った"; fail=1
elif [ "$(stay_raw "$S3")" = "$KEPT" ]; then
  echo "  OK 解析済みを残して原文だけ空にする書き換えは拒まれ、本文は残っている"
else
  echo "  NG 拒まれたのに本文が変わっている"; fail=1
fi

# 台帳つきの消去（前の版なし）は通る。そのうえで書き戻しを試す
S4="d7000000-0000-4000-8000-000000000004"; put_stay "$S4"
if psql -c "$(ledger_for "$S4") $(ERASE_SQL "$S4")" >/dev/null 2>&1 && [ -z "$(stay_raw "$S4")" ]; then
  echo "  OK 台帳つきの滞在の本文の消去は通る"
else
  echo "  NG 台帳つきの滞在の本文の消去が通らない"; fail=1
fi
# Scenario: 本文を消した滞在へ本文を書き戻せない
if psql -c "UPDATE core.event SET raw = '$KEPT' WHERE id = '$S4';" >/dev/null 2>&1; then
  echo "  NG 本文を消した滞在へ本文を書き戻せた"; fail=1
elif [ -z "$(stay_raw "$S4")" ]; then
  echo "  OK 本文を消した滞在へ本文を書き戻せず、原文は空のまま"
else
  echo "  NG 拒まれたのに原文が空でなくなっている"; fail=1
fi

# Scenario: 作り直しの書き換えは今までどおり通る
S5="d7000000-0000-4000-8000-000000000005"; put_stay "$S5"
NEW='{"end":"2026-10-01T01:00:00Z"}'
if psql -c "INSERT INTO core.event_version
              (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
            SELECT id, user_id, logical_source, 1, event_time, content_hash, raw, payload
              FROM core.event WHERE id = '$S5';
            UPDATE core.event SET raw = '$NEW', payload = '$NEW'::jsonb WHERE id = '$S5';" >/dev/null 2>&1 \
   && [ "$(stay_raw "$S5")" = "$NEW" ]; then
  echo "  OK 作り直しの書き換えは通り、前の版が積まれた"
else
  echo "  NG 作り直しの書き換えが通らない"; fail=1
fi

# 消去の操作の台帳（操作と範囲の 2 表）。行を置いたまとまりごと巻き戻す形で、書き換え・削除・切り詰めを撃つ
OP="d7000000-0000-4000-8000-0000000000a1"
put_op="INSERT INTO core.erasure_operation
          (id, user_id, kind, stay_id, start_at, end_at, stays, locations, archive_locations, archive_items, versions)
        VALUES ('$OP', '$DU', 'user', '$OP', '2026-10-01T00:00:00Z', '2026-10-01T01:00:00Z', 0, 0, 0, 0, 0);
        INSERT INTO core.erasure_span (operation_id, event_id, scope, start_at, end_at)
        VALUES ('$OP', '$OP', 'event', '2026-10-01T00:00:00Z', '2026-10-01T01:00:00Z');"
# Scenario: 消去の操作の台帳は書き換えられない
for stmt in "UPDATE core.erasure_operation SET stays = 1 WHERE id = '$OP';" \
            "UPDATE core.erasure_span SET end_at = end_at + interval '1 second' WHERE operation_id = '$OP';"; do
  if psql -c "$put_op $stmt" >/dev/null 2>&1; then
    echo "  NG 消去の操作の台帳が書き換えられた: ${stmt%% WHERE*}"; fail=1
  else
    echo "  OK 消去の操作の台帳の書き換えを拒んだ: ${stmt%% SET*}"
  fi
done
# Scenario: 消去の操作の台帳は消せず、表も切り詰められない
for stmt in "DELETE FROM core.erasure_operation WHERE id = '$OP';" \
            "DELETE FROM core.erasure_span WHERE operation_id = '$OP';" \
            "TRUNCATE core.erasure_operation;" \
            "TRUNCATE core.erasure_span;"; do
  if psql -c "$put_op $stmt" >/dev/null 2>&1; then
    echo "  NG 消去の操作の台帳を消せた・切り詰められた: ${stmt%% WHERE*}"; fail=1
  else
    echo "  OK 消去の操作の台帳を消せず、切り詰められない: ${stmt%% WHERE*}"
  fi
done
