# ST51: 更新の履歴の書き換え禁止（design D10）。tools/check-immutable.sh が `.` で読む（psql・app_psql・fail が使える）。
# 所有者とアプリの役割の両方から、更新の履歴の UPDATE・DELETE・TRUNCATE を撃って断られることを見る。
# 節は古い門の上で走るので、全版を当てた状態へ作り直す（tools/check-immutable.d/README.md。付与も当て直す）。
psql -c "DROP SCHEMA core CASCADE;" >/dev/null
for m in "${MIGS[@]}"; do psql < "migrations/$m.sql" >/dev/null; done
./tools/db-roles.sh >/dev/null
docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato_owner -d ashiato < crates/server/src/grants.sql >/dev/null
echo "== ST51: 更新の履歴に 1 行置く"
psql -c "INSERT INTO core.update_history (component, via, to_version, outcome)
         VALUES ('S-01','migrate','0.1.0+st51check','applied');" >/dev/null

echo "== Scenario: 更新の履歴の行は書き換えられない"
for st51_who in owner app; do
  for st51_sql in "UPDATE core.update_history SET to_version='x'" "DELETE FROM core.update_history" "TRUNCATE core.update_history"; do
    if [ "$st51_who" = owner ]; then
      st51_out=$(psql -c "$st51_sql;" 2>&1 >/dev/null) && st51_ok=1 || st51_ok=0
    else
      st51_out=$(app_psql -c "$st51_sql;" 2>&1 >/dev/null) && st51_ok=1 || st51_ok=0
    fi
    if [ "$st51_ok" = 1 ]; then
      echo "  NG $st51_who から更新の履歴が書き換えられた: $st51_sql"; fail=1
    elif [ "$st51_who" = owner ] && [[ "$st51_out" != *"更新の履歴は書き換えられない"* ]]; then
      echo "  NG $st51_who の拒否が門の名指しでない: $st51_sql: $st51_out"; fail=1
    else
      echo "  OK $st51_who からの拒否: $st51_sql"
    fi
  done
done
[ "$(psql -c "SELECT count(*)||'/'||min(to_version) FROM core.update_history;")" = "1/0.1.0+st51check" ] \
  || { echo "  NG 拒まれたのに更新の履歴が変わっている"; fail=1; }
