# seed.sh の Story の節（tools/seed.d/ の決まりは README.md）。ST13
# ------------------------------------------------------------------ 読み出せなかった書庫（ST13 / design D8）
#
# 台帳に、形の対応と別の種別 `file_unreadable` で読めなかった 1 冊を足す。
# **finished_at を ST12 の読めた書庫（seed.sh の「書庫（ST12）」）より古くする** —— 「直近に置いた書庫」は
# finished_at が新しい順なので、新しくすると既存の e2e（latest-archive.spec）が見る書庫を奪う。
# 何度流しても同じ（sha256 が固定で ON CONFLICT DO NOTHING）。
if [ "$MODE" = "normal" ]; then
  docker compose exec -T db psql -q -U ashiato -d ashiato -c "
  INSERT INTO core.archive_ledger
    (user_id, sha256, parser_version, outcome, created_at, file_name, inbox_kind, unreadable_kind, finished_at)
  VALUES ('00000000-0000-0000-0000-000000000000', repeat('2', 64), 'seed', 'unreadable', '2026-09-01T00:00:00Z',
          'Timeline.json', 'inbox', 'file_unreadable', '2026-09-01T00:00:00Z')
  ON CONFLICT DO NOTHING;"
  echo "normal: 読み出せなかった書庫（file_unreadable）の台帳の行を入れた"
fi
