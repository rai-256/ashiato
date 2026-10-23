# ST29: 衛星の登録簿と資格情報は消せない・読み出しの記録の衛星の行は書き換えられない・戻し手順が当たる
# （design D7 / D9。`tools/check-immutable.sh` が `.` で読む。psql・fail・MIGS は本体の変数）。
# アプリを通さず psql から直に撃つ —— 素通りする経路がここだから。

# 拒まれたときだけ OK を出す（通ってしまったら NG）。stderr に理由が出るので、その文言も見る。
refused() { # <表示名> <SQL> <理由に含まれる語>
  local err
  if err=$(psql -c "$2" 2>&1); then
    echo "  NG $1 が通ってしまった"; fail=1
  elif printf '%s' "$err" | grep -q "$3"; then
    echo "  OK $1 を拒んだ"
  else
    echo "  NG $1 が別の理由で落ちた: $err"; fail=1
  fi
}

echo "== 衛星の行を置く（衛星「読書ログ」に資格情報 A と B）"
SAT=5a7e1117-0000-4000-8000-0000000000a0
CA=5a7e1117-0000-4000-8000-0000000000a1
CB=5a7e1117-0000-4000-8000-0000000000a2
psql -c "INSERT INTO core.satellite (id, identifier, name, registered_at)
         VALUES ('$SAT', 'dev.immutable-check.booklog', '読書ログ', now());
         INSERT INTO core.satellite_credential (id, satellite_id, token_sha256, issued_at) VALUES
           ('$CA', '$SAT', '\\x0a', now()), ('$CB', '$SAT', '\\x0b', now());" >/dev/null

# 衛星を外し、A を取り消し、B は衛星を外した時刻で取り消す（口がすること。ここでは UPDATE が通ることも見る）
psql -c "UPDATE core.satellite_credential SET revoked_at = now() WHERE id = '$CA';
         UPDATE core.satellite SET removed_at = now() WHERE id = '$SAT';
         UPDATE core.satellite_credential SET revoked_at = (SELECT removed_at FROM core.satellite WHERE id = '$SAT')
          WHERE id = '$CB';" >/dev/null \
  && echo "  OK 衛星を外す・資格情報を取り消す UPDATE は通る" \
  || { echo "  NG 登録簿の UPDATE が拒まれた（外す・取り消すが書けない）"; fail=1; }

# Scenario: 外した衛星と取り消した資格情報の行は消えずに残る
for op in "satellite DELETE:DELETE FROM core.satellite WHERE id = '$SAT'" \
          "satellite TRUNCATE:TRUNCATE core.satellite CASCADE" \
          "satellite_credential DELETE:DELETE FROM core.satellite_credential WHERE satellite_id = '$SAT'" \
          "satellite_credential TRUNCATE:TRUNCATE core.satellite_credential"; do
  refused "${op%%:*}" "${op#*:}" "消せない"
done
kept=$(psql -c "SELECT (SELECT count(*) FROM core.satellite WHERE id = '$SAT' AND removed_at IS NOT NULL)
                 || ',' || (SELECT count(*) FROM core.satellite_credential
                             WHERE satellite_id = '$SAT' AND revoked_at IS NOT NULL);")
[ "$kept" = "1,2" ] || { echo "  NG 外した衛星と取り消した資格情報の行が $kept（1,2 のはず）"; fail=1; }
same=$(psql -c "SELECT count(*) FROM core.satellite_credential c JOIN core.satellite s ON s.id = c.satellite_id
                 WHERE c.id = '$CB' AND c.revoked_at = s.removed_at;")
[ "$same" = "1" ] || { echo "  NG B の取り消した時刻が衛星を外した時刻でない"; fail=1; }
[ "$kept" = "1,2" ] && [ "$same" = "1" ] && echo "  OK 外した衛星と取り消した資格情報の行は残り、時刻の列が埋まっている"

# (c) 戻し手順を、戻しの逆順の先頭で当てる（衛星の行が読み出しの記録に無い DB で。(b) の前に撃つ）。当て直せることも見る
echo "== 戻し手順（…_satellite_credentials.down.sql）を当てて、当て直す"
if psql < migrations/202610241200_satellite_credentials.down.sql >/dev/null 2>&1; then
  gone=$(psql -c "SELECT to_regclass('core.satellite') IS NULL AND to_regclass('core.satellite_credential') IS NULL;")
  cols=$(psql -c "SELECT count(*) FROM information_schema.columns
                   WHERE table_schema='core' AND table_name='access_log' AND column_name LIKE 'satellite%';")
  [ "$gone" = "t" ] && [ "$cols" = "0" ] \
    && echo "  OK 戻し手順が当たり、衛星の表と読み出しの記録の欄が消えた" \
    || { echo "  NG 戻し手順の後も衛星の表か欄が残っている"; fail=1; }
else
  echo "  NG 戻し手順が当たらない"; fail=1
fi
for round in 1 2; do
  psql < migrations/202610241200_satellite_credentials.sql >/dev/null 2>&1 \
    || { echo "  NG 戻した後の移行の当て直し（$round 回目）が落ちた"; fail=1; }
done
two=$(psql -c "SELECT count(*) FROM core.satellite WHERE builtin;")
[ "$two" = "2" ] && echo "  OK 戻した後に当て直せて、最初の 2 つは 2 行のまま" \
  || { echo "  NG 戻した後の当て直しで最初の 2 つが $two 行"; fail=1; }

# (b) 衛星の値を持つ読み出しの記録の行は、書き換えも削除も切り詰めもできない
# Scenario: 読み出しの記録の行は書き換えも削除もできない
echo "== 読み出しの記録に衛星の行を 1 件置く"
BUILTIN=5a7e1117-0000-4000-8000-000000000001
psql -c "INSERT INTO core.access_log
           (via, credential, route, method, outcome, status, satellite_id, satellite_credential_id)
         VALUES ('direct', 'satellite', '/immutable-check-satellite', 'GET', 'forbidden', 403,
                 '$BUILTIN', '$CA');" >/dev/null \
  || { echo "  NG 衛星の行を読み出しの記録に置けない"; fail=1; }
for stmt in "UPDATE core.access_log SET status = 200 WHERE credential = 'satellite'" \
            "DELETE FROM core.access_log WHERE credential = 'satellite'" \
            "TRUNCATE core.access_log"; do
  refused "satellite access_log ${stmt%% *}" "$stmt" "読み出しの記録"
done
kept=$(psql -c "SELECT count(*) FROM core.access_log
                 WHERE route = '/immutable-check-satellite' AND status = 403 AND satellite_id = '$BUILTIN';")
[ "$kept" = "1" ] || { echo "  NG 衛星の行が $kept 行になっている（1 行のはず）"; fail=1; }
