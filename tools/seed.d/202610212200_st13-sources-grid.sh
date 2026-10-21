# seed.sh の Story の節（tools/seed.d/ の決まりは README.md）。ST13
# ------------------------------------------------------------------ 稼働状況の格子の見出しに添える取り方と間隔（design D10）
#
# 書庫のソースの「最後の書庫」の経過を、**いまからの相対**で置く（通知は最後の書庫 + 75 日）。
#   視聴履歴          53 日前        → あと 22 日
#   検索履歴          80 日前        → 通知の日を過ぎた
#   タイムラインの訪問 74 日 23 時間前 → あと 1 日（鳴る 1 時間前）
#   タイムラインの移動 75 日 1 時間前  → 通知の日を過ぎた（鳴った 1 時間後）
#   Chrome の履歴     書庫も記録も無い → まだ書庫が無い（このファイルは何も入れない）
#   （退役した移行前のロケーション履歴は seed に置かない —— 退役した格子は畳まれ、書庫の格子の実寸を測る
#    既存の e2e（archive-layout）が週の帯 0 本で落ちる。e2e が応答の retired_on を差し替える）
# 台帳の finished_at は created_at と同じ古い値にする —— 「直近に置いた書庫」は finished_at の新しい順なので、
# 既定の now() にすると既存の e2e（latest-archive.spec）が見る書庫を奪う。
if [ "$MODE" = "normal" ]; then
  docker compose exec -T db psql -q -U ashiato -d ashiato -v ON_ERROR_STOP=1 <<'SQL'
-- 視聴履歴（53 日前）は seed.sh（ST12）の書庫の行がそのまま持つ（台帳は追記のみで UPDATE できない）
WITH ledger AS (
  INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, created_at, file_name, inbox_kind, finished_at)
  SELECT '00000000-0000-0000-0000-000000000000', v.sha, 'seed', 'read', now() - v.age, v.name, 'inbox', now() - v.age
    FROM (VALUES (repeat('3', 64), interval '80 days',                 'takeout-st13-search.zip'),
                 (repeat('4', 64), interval '74 days 23 hours',        'takeout-st13-visit.zip'),
                 (repeat('5', 64), interval '75 days 1 hour',          'takeout-st13-move.zip')) AS v(sha, age, name)
  ON CONFLICT DO NOTHING RETURNING id, sha256
)
INSERT INTO core.archive_ledger_source (ledger_id, logical_source, inserted_count)
SELECT l.id,
       CASE l.sha256 WHEN repeat('3', 64) THEN 'c03-youtube-search'
                     WHEN repeat('4', 64) THEN 'c03-timeline-visit'
                     ELSE 'c03-timeline-move' END, 1
  FROM ledger l ON CONFLICT DO NOTHING;

-- 位置の生存信号の原文。休止の対象から外れている状態（`exempt`）。外れていない版は e2e が /api/sources の
-- 応答の device_notice を差し替えて確かめる。**日は 2026-09-07（位置の記録がある日）に置く** —— 別の日に置くと
-- 「動いていた・記録なし」の格子の色が増える
INSERT INTO core.heartbeat (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
                            attempts, successes, content_hash, raw)
VALUES (gen_random_uuid(), '00000000-0000-0000-0000-000000000000', 'c01-location', 'seed-phone',
        '2026-09-07T10:00:00Z', true, '{}', 1, 1, 'seed-st13-unused-app-restrictions',
        '{"unused_app_restrictions":"exempt"}')
ON CONFLICT DO NOTHING;
SQL
  echo "normal: 書庫の経過（53 日 / 80 日 / 74 日 23 時間 / 75 日 1 時間）・位置の生存信号の除外の状態を入れた"
fi
