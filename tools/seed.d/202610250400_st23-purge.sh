# seed.sh の Story の節（tools/seed.d/ の決まりは README.md）。
# ------------------------------------------------------------------ 本文ごと消す確認に出る「書庫の写し」（ST23）
#
# 「本文ごと消す」の確認は、消えずに残る書庫の写しの数を出す（置き場にファイルが残っているものを数える。design D10）。
# e2e（web/e2e/st23-purge.spec.ts）が読む材料として、ロケーション履歴の写しのファイルを 2 個置き、目録の行を足す。
# **読むだけの材料。何も消さないので何度流しても同じ**（目録は追記のみ・鍵は (利用者, 中身の sha256)。2 回目は何も足さない）。
# どの書庫から出た写しか分からない目録（archive_sha256 なし）は、ソースを否定できないので必ず数える（D10）。
# 書庫の台帳には行を足さない（書庫の画面の直近結果・件数を動かさない）。
st23_copies="${ASHIATO_SEED_FILES:-/var/tmp/ashiato-seed-files}/st23"
mkdir -p "$st23_copies"
for n in 1 2; do
  file="$st23_copies/Location History $n.json"
  [ -f "$file" ] || printf '{"locations":[],"copy":%s}\n' "$n" > "$file"
  digest=$(sha256sum "$file" | cut -d' ' -f1)
  docker compose exec -T db psql -q -U ashiato -d ashiato \
    -v path="$file" -v digest="$digest" -v name="Location History $n.json" <<'SQL' >/dev/null
INSERT INTO core.archive_file (sha256, user_id, inner_path, stored_path)
VALUES (:'digest', '00000000-0000-0000-0000-000000000000', :'name', :'path')
ON CONFLICT DO NOTHING;
SQL
done
copies=$(docker compose exec -T db psql -qtAX -U ashiato -d ashiato -c \
  "SELECT count(*) FROM core.archive_file WHERE user_id = '00000000-0000-0000-0000-000000000000'
      AND stored_path LIKE '$st23_copies/%';" | tr -d '[:space:]')
[ "$copies" -eq 2 ] || { echo "normal: 書庫の写しの目録が 2 行でない（$copies 行）"; exit 1; }
echo "normal: 書庫の写し 2 個（$st23_copies）を置き、目録に足した（本文ごと消す確認が数える材料）"
