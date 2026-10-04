-- 確認待ちのファイルの中身のハッシュと、確認待ちのために写しを作ったか（ST12 / final review R51 / R56）。
-- 直近の走査で置き場が読めたか（final review R59 / design D19（仮））。
-- 前進のみ。戻し手順は 202610042315_archive_pending_file.down.sql に置く。
--
-- **中身のハッシュで写しを引く。** 書庫のハッシュで写しの目録を引いていたときは、中身が同じ
-- ファイルを持つ 2 冊目の書庫の目録が `core.archive_file` の鍵 `(user_id, sha256)` で落ち、
-- 印を置いても 2 冊目が永久に確認待ちに残った。
-- **写しを作ったのが確認待ちのためだったか**を持つ。残さない設定のとき、読み直した後に
-- 消してよいのはこの写しだけ（既に作った写しは残す。第 2 回 Q11 / spec）。
-- `core.archive_pending_shape` は書き換えてよい観測値なので、列を足すだけで済む。
ALTER TABLE core.archive_pending_shape ADD COLUMN IF NOT EXISTS file_sha256 text;
ALTER TABLE core.archive_pending_shape ADD COLUMN IF NOT EXISTS made_copy boolean NOT NULL DEFAULT false;

-- **直近の走査で置き場が読めたか**。生存信号は日に 1 回なので、箱が信号だけを見ていると、
-- 昼に置き場が読めなくなっても翌日まで出なかった（design D19（仮））。
-- `core.archive_scan_counter` は書き換えてよい 1 行の表なので、列を足すだけで済む。
ALTER TABLE core.archive_scan_counter ADD COLUMN IF NOT EXISTS last_capturable boolean;
ALTER TABLE core.archive_scan_counter ADD COLUMN IF NOT EXISTS last_blockers text[] NOT NULL DEFAULT '{}';
