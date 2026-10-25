# ST48（design D10 / review/code.md R23）: 版の記録の置き場（bootstrap）だけを作る。行は書かない —— psql で当てた版は記録しない。
# これから足す版（名前の時刻が 202610250000 以後）の .down.sql は記録の行を消すので、置き場が無いと後ろの戻しの検査が落ちる。
# 後ろの節と本体の戻しの並びより前に読まれるよう、名前の時刻を 202610250000 にしてある（それ以後の版を持つ Story の節はこれより後ろに並ぶ）。
# 所有者で当てる（`migration_ledger.sql` の末尾の付与は、節の前の準備が作ったアプリの役割へ向く）。
docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato_owner -d ashiato < crates/server/src/migration_ledger.sql >/dev/null
if app_psql -c "SELECT count(*) FROM migration_ledger.applied;" >/dev/null; then
  echo "  OK 版の記録の置き場がある（アプリの役割から読める）"
else
  echo "  NG 版の記録の置き場が無いか、アプリの役割から読めない"; fail=1
fi
