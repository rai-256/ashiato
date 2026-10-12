-- 202610042315_archive_pending_file.sql の戻し。確認待ちの観測値の列だけを落とす
-- （書き換えてよい表。落とすと、2 冊目の確認待ちは書庫のハッシュでしか写しを引けなくなる）。
ALTER TABLE core.archive_pending_shape DROP COLUMN IF EXISTS made_copy;
ALTER TABLE core.archive_pending_shape DROP COLUMN IF EXISTS file_sha256;
ALTER TABLE core.archive_scan_counter DROP COLUMN IF EXISTS last_blockers;
ALTER TABLE core.archive_scan_counter DROP COLUMN IF EXISTS last_capturable;
