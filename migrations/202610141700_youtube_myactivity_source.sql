-- マイアクティビティの YouTube のうち、視聴でも検索でもない項目の行き先（ST47 / design D8）。
-- 書庫を置く前から登録しておくのは、格子に「まだ無い」と出して、分けた行き先があることを本人が見られるようにするため。
-- 見出しは `worker::ensure_myactivity_source` が作るものと同じ。
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。本人が変えた想定間隔は戻さない。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('c03-myactivity-youtube', 'マイアクティビティ: YouTube', 5184000, 'none')
ON CONFLICT (logical_source) DO NOTHING;
